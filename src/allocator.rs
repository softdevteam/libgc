use std::{
    alloc::{AllocErr, AllocRef, GlobalAlloc, Layout},
    ffi::c_void,
    ptr::NonNull,
};

use crate::boehm;

pub struct GlobalAllocator;
pub struct GcAllocator;

unsafe impl GlobalAlloc for GlobalAllocator {
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

unsafe impl AllocRef for GcAllocator {
    fn alloc(&mut self, layout: Layout) -> Result<(NonNull<u8>, usize), AllocErr> {
        let ptr = unsafe { boehm::GC_malloc(layout.size()) } as *mut u8;
        assert!(!ptr.is_null());
        Ok((unsafe { NonNull::new_unchecked(ptr) }, layout.size()))
    }

    unsafe fn dealloc(&mut self, _: NonNull<u8>, _: Layout) {}

    unsafe fn realloc(
        &mut self,
        ptr: NonNull<u8>,
        _layout: Layout,
        new_size: usize,
    ) -> Result<(NonNull<u8>, usize), AllocErr> {
        let cptr = ptr.as_ptr() as *mut c_void;
        Ok((
            NonNull::new_unchecked(boehm::GC_realloc(cptr, new_size) as *mut u8),
            new_size,
        ))
    }
}
