use std::{
    alloc::{AllocErr, AllocInit, AllocRef, GlobalAlloc, Layout, MemoryBlock},
    ffi::c_void,
    ptr::NonNull,
};

use crate::boehm;

pub struct BoehmAllocator;
pub(crate) struct BoehmGcAllocator;

unsafe impl GlobalAlloc for BoehmAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        boehm::GC_malloc_uncollectable(layout.size()) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        boehm::GC_free(ptr as *mut c_void);
    }

    unsafe fn realloc(&self, ptr: *mut u8, _: Layout, new_size: usize) -> *mut u8 {
        boehm::GC_realloc(ptr as *mut c_void, new_size) as *mut u8
    }
}

unsafe impl AllocRef for BoehmGcAllocator {
    fn alloc(&mut self, layout: Layout, _init: AllocInit) -> Result<MemoryBlock, AllocErr> {
        let ptr = unsafe { boehm::GC_malloc(layout.size()) } as *mut u8;
        assert!(!ptr.is_null());
        Ok(MemoryBlock {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            size: layout.size(),
        })
    }

    unsafe fn dealloc(&mut self, _: NonNull<u8>, _: Layout) {}
}
