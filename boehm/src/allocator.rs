use std::{
    alloc::{AllocError, AllocRef, GlobalAlloc, Layout},
    ptr::NonNull,
};

use crate::ffi;

pub struct BoehmAllocator;
pub struct BoehmGcAllocator;
pub struct PreciseAllocator;

unsafe impl GlobalAlloc for BoehmAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ffi::GC_malloc_uncollectable(layout.size()) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        ffi::GC_free(ptr);
    }

    unsafe fn realloc(&self, ptr: *mut u8, _: Layout, new_size: usize) -> *mut u8 {
        ffi::GC_realloc(ptr, new_size) as *mut u8
    }
}

unsafe impl AllocRef for BoehmGcAllocator {
    fn alloc(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = unsafe { ffi::GC_malloc(layout.size()) } as *mut u8;
        assert!(!ptr.is_null());
        let ptr = unsafe { NonNull::new_unchecked(ptr) };
        Ok(NonNull::slice_from_raw_parts(ptr, layout.size()))
    }

    unsafe fn dealloc(&self, _: NonNull<u8>, _: Layout) {}
}

impl PreciseAllocator {
    pub unsafe fn alloc_partially_traceable(
        &self,
        layout: Layout,
        boundary: usize,
    ) -> Result<NonNull<[u8]>, AllocError> {
        // FIXME: Right now, this will only work for blocks smaller than 4KB.
        assert!(layout.size() <= 4096);

        // The idea here is simple. A bitmap is used to denote whether a
        // particular word in an allocation block may hold a pointer. Each bit
        // corresponds to a single word. A 1 indicates that a word *may* be a
        // pointer, a 0 indicates that it definitely isn't.
        //
        // The boundary tells Boehm how far into the bitmap it should read. For
        // example, a boundary of 4 means that only the first 4 bits in the
        // bitmap are relevant, therefore only the first 4 words in the block
        // are traced as candidate pointers.
        //
        // By setting all bits to 1, and specifying a boundary, we can take
        // advantage of precise layout support to reduce the traceable region,
        // without the complexity of having to provide accurate layouts for each
        // type.
        let bitmap: usize = 0xFFFFFFFF;
        let gc_descr = ffi::GC_make_descriptor(&bitmap as *const usize, boundary);

        let ptr = ffi::GC_malloc_explicitly_typed(layout.size(), gc_descr);
        let ptr = NonNull::new_unchecked(ptr);
        Ok(NonNull::slice_from_raw_parts(ptr, layout.size()))
    }
}
