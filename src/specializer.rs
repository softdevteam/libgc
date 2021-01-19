//! The contents of this module require the rustgc feature. To avoid polluting
//! lib.rs with conditional compilation attributes, it is only exported if the
//! rustgc feature is enabled.
use core::{
    alloc::{AllocError, Layout},
    gc::{Conservative, NoTrace},
    marker::PhantomData,
    ptr::NonNull,
};

#[cfg(test)]
use core::cell::Cell;

use crate::boehm;
use crate::ALLOCATOR;

/// An allocation helper to potentially optimise the marking of allocated
/// blocks.
///
/// Allocation which is done through this bottleneck will try to use type
/// information to allocate blocks in pools which are better optimised for
/// marking. Allocation is a hot path, so to reduce branching at runtime, it
/// makes heavy use of specialization, hence the name. Specialized allocation
/// can be thought of as "compile-time cascading if-else".
///
/// There are three ways a block can be allocated:
///
///    Atomically - this means that a block contains no pointers which need
///    tracing during marking. An atomically allocated block is a leaf node in
///    the object graph.
///
///    Precisely - blocks are allocated along with a bitmap which describes
///    where the pointers are on a per-word basis. The parts of a block which
///    are not described in the bitmap are ignored by the collector during
///    marking.
///
///    Conservatively - the whereabouts of pointers inside the block are
///    unknown, and each word in the block must be considered for potential
///    pointers.
///
/// There is a direct trade-off between the safety and performance of these
/// three allocation types: atomic allocation will have the fastest marking
/// times, but is the most dangerous; whereas conservative allocation is safe to
/// use anywhere, but will have the slowest marking times.
///
/// This approach, however, only works when an allocation block corresponds to
/// the layout of a single type. This isn't always the case (hence, the
/// allocator_api operates on a Layout, which deliberately carries less
/// information than a type parameter). For example, it's perfectly legal in
/// Rust to allocate a chunk of memory, and use it to store values of type X, Y,
/// and Z contiguously (provided size and alignment constraints are met).
/// Conservative allocation is currently the only safe way to allocate such
/// blocks.
#[cfg(test)]
pub(crate) struct AllocationSpecializer {
    num_atomic: Cell<usize>,
    num_precise: Cell<usize>,
    num_conservative: Cell<usize>,
}

#[cfg(not(test))]
pub(crate) struct AllocationSpecializer;

impl AllocationSpecializer {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            num_atomic: Cell::new(0),
            num_precise: Cell::new(0),
            num_conservative: Cell::new(0),
        }
    }

    #[cfg(not(test))]
    pub(crate) fn new() -> Self {
        Self
    }

    #[inline]
    pub(crate) fn maybe_optimised_alloc<T>(
        &self,
        layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        ConservativeSpecializer::<T>::maybe_optimised_alloc(self, layout)
    }

    /// Allocates a block of memory for `layout` which is guaranteed to contains
    /// no GC pointers.
    ///
    /// An atomic allocation has the same semantics as regular `alloc` except
    /// for one key difference which is that it tells the collector not to trace
    /// this block during marking.
    ///
    /// # Safety
    ///
    /// The block must not contain any pointers which, either directly or
    /// transitively, point to a GC'd value.
    unsafe fn atomic_alloc(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = boehm::GC_malloc_atomic(layout.size()) as *mut u8;
        let ptr = NonNull::new_unchecked(ptr);
        #[cfg(test)]
        {
            self.num_atomic.set(self.num_atomic.get() + 1);
        }
        Ok(NonNull::slice_from_raw_parts(ptr, layout.size()))
    }

    /// Allocates a block of memory for `layout` where word-aligned fields are
    /// described to the collector by `bitmap`.
    ///
    /// An atomic allocation has the same semantics as regular `alloc` except
    /// for one key difference which is that it tells the collector not to trace
    /// this block during marking.
    ///
    /// # Safety
    ///
    /// The size described by `layout` must be no larger than 4092 bytes.
    ///
    /// An incorrect `bitmap` will lead to undefined behaviour.
    ///
    /// The size described by `layout` must be at least as big as
    /// `size_of::<usize> * bitmap_size`.
    ///
    /// The returned pointer must not be passed to `realloc` under any
    /// circumstances.
    unsafe fn precise_alloc(
        &self,
        layout: Layout,
        bitmap: u64,
        bitmap_size: u64,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let gc_descr =
            boehm::GC_make_descriptor(&bitmap as *const u64 as *const usize, bitmap_size as usize);
        let ptr = boehm::GC_malloc_explicitly_typed(layout.size(), gc_descr);
        let ptr = NonNull::new_unchecked(ptr);
        #[cfg(test)]
        {
            self.num_precise.set(self.num_precise.get() + 1);
        }
        Ok(NonNull::slice_from_raw_parts(ptr, layout.size()))
    }

    /// Allocates a block of memory for `layout` which is traced conservatively
    /// by the collector during marking.
    fn conservative_alloc(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = unsafe { boehm::GC_malloc(layout.size()) } as *mut u8;
        let ptr = unsafe { NonNull::new_unchecked(ptr) };
        #[cfg(test)]
        {
            self.num_conservative.set(self.num_conservative.get() + 1);
        }
        Ok(NonNull::slice_from_raw_parts(ptr, layout.size()))
    }
}

/// Specializes allocation for `T` based on its impl of the `Conservative` trait.
///
/// The implementation of `T: Conservative` will bypass all other allocation
/// strategies and guarantee that the block required by `layout` is allocated
/// conservatively.
trait ConservativeSpecializer<T> {
    fn maybe_optimised_alloc(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError>;
}

impl<T> ConservativeSpecializer<T> for AllocationSpecializer {
    default fn maybe_optimised_alloc(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        AtomicSpecializer::<T>::maybe_optimised_alloc(self, layout)
    }
}

impl<T: Conservative> ConservativeSpecializer<T> for AllocationSpecializer {
    fn maybe_optimised_alloc(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.conservative_alloc(layout)
    }
}

/// Specializes allocation for `T` based on its impl of the `NoTrace` trait.
///
/// The implementation of `T: NoTrace` guarantees (provided that `T:
/// !Conservative`) that the block required by `layout` is allocated atomically.
trait AtomicSpecializer<T> {
    fn maybe_optimised_alloc(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError>;
}

impl<T> AtomicSpecializer<T> for AllocationSpecializer {
    default fn maybe_optimised_alloc(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let trace = unsafe { ::core::gc::gc_layout::<T>() };
        if trace.must_use_conservative() {
            return self.conservative_alloc(layout);
        }
        unsafe { self.precise_alloc(layout, trace.bitmap, trace.size) }
    }
}

impl<T: NoTrace> AtomicSpecializer<T> for AllocationSpecializer {
    fn maybe_optimised_alloc(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { self.atomic_alloc(layout) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_alloc() {
        struct T(usize);
        impl NoTrace for T {}

        struct U(usize);
        impl !NoTrace for U {}

        let sp = AllocationSpecializer::new();

        sp.maybe_optimised_alloc::<T>(Layout::new::<T>());
        assert_eq!(sp.num_atomic.get(), 1);

        sp.maybe_optimised_alloc::<U>(Layout::new::<U>());
        assert_eq!(sp.num_atomic.get(), 1);
    }

    #[test]
    fn conservative_alloc() {
        struct T(usize);

        impl NoTrace for T {}
        impl Conservative for T {}

        let sp = AllocationSpecializer::new();

        sp.maybe_optimised_alloc::<T>(Layout::new::<T>());
        assert_eq!(sp.num_conservative.get(), 1);
    }

    #[test]
    fn precise_alloc() {
        struct T(usize);

        impl !NoTrace for T {}

        let sp = AllocationSpecializer::new();
        sp.maybe_optimised_alloc::<T>(Layout::new::<T>());
        assert_eq!(sp.num_precise.get(), 1);
    }
}
