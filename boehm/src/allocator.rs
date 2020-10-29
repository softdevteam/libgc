use std::{
    alloc::{AllocError, AllocRef, GlobalAlloc, Layout},
    ptr::NonNull,
};

use crate::ffi;

pub struct BoehmAllocator;
pub struct BoehmGcAllocator;

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
