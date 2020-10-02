use std::{
    alloc::{AllocError, AllocRef, GlobalAlloc, Layout},
    ptr::NonNull,
};

use crate::boehm;

pub struct BoehmAllocator;
pub(crate) struct BoehmGcAllocator;

unsafe impl GlobalAlloc for BoehmAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        boehm::gc_malloc_uncollectable(layout.size()) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        boehm::gc_free(ptr);
    }

    unsafe fn realloc(&self, ptr: *mut u8, _: Layout, new_size: usize) -> *mut u8 {
        boehm::gc_realloc(ptr, new_size) as *mut u8
    }
}

unsafe impl AllocRef for BoehmGcAllocator {
    fn alloc(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = unsafe { boehm::gc_malloc(layout.size()) } as *mut u8;
        assert!(!ptr.is_null());
        let ptr = unsafe { NonNull::new_unchecked(ptr) };
        Ok(NonNull::slice_from_raw_parts(ptr, layout.size()))
    }

    unsafe fn dealloc(&self, _: NonNull<u8>, _: Layout) {}
}
