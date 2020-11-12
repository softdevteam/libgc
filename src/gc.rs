use std::{
    alloc::{AllocError, AllocRef, Layout},
    any::Any,
    fmt,
    hash::{Hash, Hasher},
    marker::{PhantomData, Unsize},
    mem::{forget, ManuallyDrop, MaybeUninit},
    ops::{CoerceUnsized, Deref, DerefMut, DispatchFromDyn},
    ptr::NonNull,
};

use crate::{GC_ALLOCATOR, PRECISE_ALLOCATOR};

/// This is usually a no-op, but if `gc_stats` is enabled it will setup the GC
/// for profiliing.
pub fn gc_init() {
    #[cfg(all(feature = "gc_stats", feature = "boehm"))]
    boehm::init();
}

/// A garbage collected pointer.
///
/// The type `Gc<T>` provides shared ownership of a value of type `T`,
/// allocted in the heap. `Gc` pointers are `Copyable`, so new pointers to
/// the same value in the heap can be produced trivially. The lifetime of
/// `T` is tracked automatically: it is freed when the application
/// determines that no references to `T` are in scope. This does not happen
/// deterministically, and no guarantees are given about when a value
/// managed by `Gc` is freed.
///
/// Shared references in Rust disallow mutation by default, and `Gc` is no
/// exception: you cannot generally obtain a mutable reference to something
/// inside an `Gc`. If you need mutability, put a `Cell` or `RefCell` inside
/// the `Gc`.
///
/// Unlike `Rc<T>`, cycles between `Gc` pointers are allowed and can be
/// deallocated without issue.
///
/// `Gc<T>` automatically dereferences to `T` (via the `Deref` trait), so
/// you can call `T`'s methods on a value of type `Gc<T>`.
#[derive(PartialEq, Eq, Debug)]
pub struct Gc<T: ?Sized> {
    ptr: NonNull<GcBox<T>>,
    _phantom: PhantomData<T>,
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Gc<U>> for Gc<T> {}
impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<Gc<U>> for Gc<T> {}

impl<T> Gc<T> {
    /// Constructs a new `Gc<T>`.
    pub fn new(v: T) -> Self {
        Gc {
            ptr: unsafe { NonNull::new_unchecked(GcBox::new(v)) },
            _phantom: PhantomData,
        }
    }

    /// Constructs a new `Gc<MaybeUninit<T>>` which is capable of storing data
    /// up-to the size permissible by `layout`.
    ///
    /// This can be useful if you want to store a value with a custom layout,
    /// but have the collector treat the value as if it were T.
    ///
    /// # Panics
    ///
    /// If `layout` is smaller than that required by `T` and/or has an alignment
    /// which is smaller than that required by `T`.
    pub fn new_from_layout(layout: Layout) -> Gc<MaybeUninit<T>> {
        let tl = Layout::new::<T>();
        if layout.size() < tl.size() || layout.align() < tl.align() {
            panic!(
                "Requested layout {:?} is either smaller than size {} and/or not aligned to {}",
                layout,
                tl.size(),
                tl.align()
            );
        }
        unsafe { Gc::new_from_layout_unchecked(layout) }
    }

    /// Constructs a new `Gc<MaybeUninit<T>>` which is capable of storing data
    /// up-to the size permissible by `layout`.
    ///
    /// This can be useful if you want to store a value with a custom layout,
    /// but have the collector treat the value as if it were T.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring that both `layout`'s size and
    /// alignment must match or exceed that required to store `T`.
    pub unsafe fn new_from_layout_unchecked(layout: Layout) -> Gc<MaybeUninit<T>> {
        Gc::from_inner(GcBox::new_from_layout(layout))
    }

    pub fn unregister_finalizer(&mut self) {
        let ptr = self.ptr.as_ptr() as *mut GcBox<T>;
        unsafe {
            GcBox::unregister_finalizer(&mut *ptr);
        }
    }
}

impl Gc<dyn Any> {
    pub fn downcast<T: Any>(&self) -> Result<Gc<T>, Gc<dyn Any>> {
        if (*self).is::<T>() {
            let ptr = self.ptr.cast::<GcBox<T>>();
            Ok(Gc::from_inner(ptr))
        } else {
            Err(Gc::from_inner(self.ptr))
        }
    }
}

#[cfg(not(feature = "rustc_boehm"))]
pub fn needs_finalizer<T>() -> bool {
    std::mem::needs_drop::<T>()
}

#[cfg(feature = "rustc_boehm")]
pub fn needs_finalizer<T>() -> bool {
    std::mem::needs_finalizer::<T>()
}

impl<T: ?Sized> Gc<T> {
    /// Get a raw pointer to the underlying value `T`.
    pub fn into_raw(this: Self) -> *const T {
        this.ptr.as_ptr() as *const T
    }

    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        this.ptr.as_ptr() == other.ptr.as_ptr()
    }

    /// Get a `Gc<T>` from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `raw` was allocated with `Gc::new()` or
    /// u8 `Gc::new_from_layout()`.
    ///
    /// It is legal for `raw` to be an interior pointer if `T` is valid for the
    /// size and alignment of the originally allocated block.
    pub fn from_raw(raw: *const T) -> Gc<T> {
        Gc {
            ptr: unsafe { NonNull::new_unchecked(raw as *mut GcBox<T>) },
            _phantom: PhantomData,
        }
    }

    fn from_inner(ptr: NonNull<GcBox<T>>) -> Self {
        Self {
            ptr,
            _phantom: PhantomData,
        }
    }
}

impl<T> Gc<MaybeUninit<T>> {
    /// As with `MaybeUninit::assume_init`, it is up to the caller to guarantee
    /// that the inner value really is in an initialized state. Calling this
    /// when the content is not yet fully initialized causes immediate undefined
    /// behaviour.
    pub unsafe fn assume_init(self) -> Gc<T> {
        let ptr = self.ptr.as_ptr() as *mut GcBox<MaybeUninit<T>>;
        Gc::from_inner((&mut *ptr).assume_init())
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for Gc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

/// A `GcBox` is a 0-cost wrapper which allows a single `Drop` implementation
/// while also permitting multiple, copyable `Gc` references. The `drop` method
/// on `GcBox` acts as a guard, preventing the destructors on its contents from
/// running unless the object is really dead.
struct GcBox<T: ?Sized>(ManuallyDrop<T>);

trait GcBoxAllocator<T> {
    fn alloc_dispatch(value: &T) -> Result<NonNull<[u8]>, AllocError>;
}

impl<T> GcBoxAllocator<T> for GcBox<T> {
    default fn alloc_dispatch(_value: &T) -> Result<NonNull<[u8]>, AllocError> {
        let layout = Layout::new::<T>();
        GC_ALLOCATOR.alloc(layout)
    }
}

impl<T: GcLayout> GcBoxAllocator<T> for GcBox<T> {
    fn alloc_dispatch(value: &T) -> Result<NonNull<[u8]>, AllocError> {
        let layout = Layout::new::<T>();
        match GcLayout::layout_info(value) {
            LayoutInfo::PartiallyTraceable(boundary) => unsafe {
                PRECISE_ALLOCATOR.alloc_partially_traceable(layout, boundary)
            },
            LayoutInfo::Conservative => GC_ALLOCATOR.alloc(layout),
            LayoutInfo::Precise { .. } => unimplemented!(),
        }
    }
}

impl<T> GcBox<T> {
    fn new(value: T) -> *mut GcBox<T> {
        let ptr = Self::alloc_dispatch(&value).unwrap().as_ptr() as *mut GcBox<T>;
        let gcbox = GcBox(ManuallyDrop::new(value));

        unsafe {
            ptr.copy_from_nonoverlapping(&gcbox, 1);
            GcBox::register_finalizer(&mut *ptr);
        }

        forget(gcbox);
        ptr
    }

    fn new_from_layout(layout: Layout) -> NonNull<GcBox<MaybeUninit<T>>> {
        unsafe {
            let base_ptr = GC_ALLOCATOR.alloc(layout).unwrap().as_ptr() as *mut usize;
            NonNull::new_unchecked(base_ptr as *mut GcBox<MaybeUninit<T>>)
        }
    }

    fn register_finalizer(&mut self) {
        #[cfg(feature = "gc_stats")]
        crate::stats::NUM_REGISTERED_FINALIZERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        #[cfg(feature = "boehm")]
        boehm::register_finalizer(self as *mut _ as *mut T);
    }

    fn unregister_finalizer(&mut self) {
        #[cfg(feature = "boehm")]
        boehm::unregister_finalizer(self as *mut _ as *mut u8);
    }
}

#[derive(PartialEq, Eq)]
pub enum LayoutInfo {
    /// A partially traceable type is conservatively traced up until a specified
    /// word boundary.
    PartiallyTraceable(usize),
    /// The default tracing mechanism. This has the same effect as not
    /// implementing `GcLayout`.
    Conservative,
    Precise {
        bitmap: usize,
        trace_len: usize,
    },
}

impl fmt::Debug for LayoutInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutInfo::Precise { bitmap, trace_len } => write!(
                f,
                "Precise{{ bitmap:{:#X}, trace_len {} }})",
                bitmap, trace_len
            ),
            LayoutInfo::Conservative => write!(f, "Conservative"),
            LayoutInfo::PartiallyTraceable(len) => write!(f, "PartiallyTraceable({})", len),
        }
    }
}

/// Used to pass more specific layout information about a type to the collector.
///
/// # Safety
///
/// This is very unsafe. Incorrect layout information can lead to the GC missing
/// pointers, resulting in unsoundness.
pub unsafe trait GcLayout {
    fn layout_info(&self) -> LayoutInfo;
}

impl<T> GcBox<MaybeUninit<T>> {
    unsafe fn assume_init(&mut self) -> NonNull<GcBox<T>> {
        // Now that T is initialized, we must make sure that it's dropped when
        // `GcBox<T>` is freed.
        let init = self as *mut _ as *mut GcBox<T>;
        GcBox::register_finalizer(&mut *init);
        NonNull::new_unchecked(init)
    }
}

impl<T: ?Sized> Deref for Gc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.ptr.as_ptr() as *const T) }
    }
}

impl<T: ?Sized> DerefMut for Gc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.ptr.as_ptr() as *mut T) }
    }
}

/// `Copy` and `Clone` are implemented manually because a reference to `Gc<T>`
/// should be copyable regardless of `T`. It differs subtly from `#[derive(Copy,
/// Clone)]` in that the latter only makes `Gc<T>` copyable if `T` is.
impl<T: ?Sized> Copy for Gc<T> {}

impl<T: ?Sized> Clone for Gc<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + Hash> Hash for Gc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::mem::size_of;

    #[test]
    #[should_panic]
    fn test_too_small() {
        Gc::<[u8; 256]>::new_from_layout(Layout::from_size_align(1, 1).unwrap());
    }

    #[test]
    #[should_panic]
    fn test_unaligned() {
        #[repr(align(1024))]
        struct S {
            _x: usize,
        }
        Gc::<S>::new_from_layout(Layout::from_size_align(size_of::<S>(), 1).unwrap());
    }

    #[test]
    fn test_dispatchable() {
        struct S1 {
            x: u64,
        }
        struct S2 {
            y: u64,
        }
        trait T {
            fn f(self: Gc<Self>) -> u64;
        }
        impl T for S1 {
            fn f(self: Gc<Self>) -> u64 {
                self.x
            }
        }
        impl T for S2 {
            fn f(self: Gc<Self>) -> u64 {
                self.y
            }
        }

        let s1 = S1 { x: 1 };
        let s2 = S2 { y: 2 };
        let s1gc: Gc<S1> = Gc::new(s1);
        let s2gc: Gc<S2> = Gc::new(s2);
        assert_eq!(s1gc.f(), 1);
        assert_eq!(s2gc.f(), 2);

        let s1gcd: Gc<T> = s1gc;
        let s2gcd: Gc<T> = s2gc;
        assert_eq!(s1gcd.f(), 1);
        assert_eq!(s2gcd.f(), 2);
    }

    #[test]
    fn precise_layout() {
        #[derive(GcLayout)]
        struct A {
            #[no_trace]
            x: usize,
            #[no_trace]
            y: usize,
            #[no_trace]
            z: usize,
        }

        #[derive(GcLayout)]
        struct B {
            x: usize,
            #[no_trace]
            y: usize,
            z: usize,
        }
        let a = A {
            x: 123,
            y: 456,
            z: 789,
        };
        assert_eq!(
            a.layout_info(),
            LayoutInfo::Precise {
                bitmap: 0xFFFFFFF8,
                trace_len: 3
            }
        );

        let b = B {
            x: 123,
            y: 456,
            z: 789,
        };
        assert_eq!(
            b.layout_info(),
            LayoutInfo::Precise {
                bitmap: 0xFFFFFFFD,
                trace_len: 3
            }
        );
    }

    #[test]
    fn precise_layout_unsized() {
        #[derive(GcLayout)]
        struct StaticallyUnsized<T: ?Sized> {
            x: usize,
            #[no_trace]
            y: T,
        }

        let s: StaticallyUnsized<[usize; 8]> = StaticallyUnsized { x: 123, y: [0; 8] };
        let dynamic: &StaticallyUnsized<[usize]> = &s;
        assert_eq!(
            dynamic.layout_info(),
            LayoutInfo::Precise {
                bitmap: 0xFFFFFE01,
                trace_len: 9
            }
        );
    }

    #[test]
    fn conservative_layout() {
        #[derive(GcLayout)]
        struct A {
            x: usize,
            y: usize,
            z: usize,
        }

        let a = A {
            x: 123,
            y: 456,
            z: 789,
        };
        assert_eq!(a.layout_info(), LayoutInfo::Conservative);
    }

    #[test]
    fn partially_traceable_layout() {
        #[derive(GcLayout)]
        struct A {
            x: usize,
            #[trace_end] // Stop tracing *AFTER* y
            y: usize,
            z: usize,
        }

        let a = A {
            x: 123,
            y: 456,
            z: 789,
        };
        assert_eq!(a.layout_info(), LayoutInfo::PartiallyTraceable(2));
    }
}
