//! This library acts as a shim to prevent static linking the Boehm GC directly
//! inside library/alloc which causes surprising and hard to debug errors.

#![allow(dead_code)]

use core::{
    alloc::{AllocError, Allocator, GlobalAlloc, Layout},
    ptr::NonNull,
};

pub struct GcAllocator;

use crate::boehm;
#[cfg(feature = "rustgc")]
use crate::specializer;

#[cfg(feature = "rustgc")]
pub(crate) static ALLOCATOR: GcAllocator = GcAllocator;

unsafe impl GlobalAlloc for GcAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        #[cfg(feature = "rustgc")]
        return boehm::GC_malloc(layout.size()) as *mut u8;
        #[cfg(not(feature = "rustgc"))]
        return boehm::GC_malloc_uncollectable(layout.size()) as *mut u8;
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        boehm::GC_free(ptr);
    }

    unsafe fn realloc(&self, ptr: *mut u8, _: Layout, new_size: usize) -> *mut u8 {
        boehm::GC_realloc(ptr, new_size) as *mut u8
    }

    #[cfg(feature = "rustgc_internal")]
    unsafe fn alloc_precise(&self, layout: Layout, bitmap: usize, bitmap_size: usize) -> *mut u8 {
        let gc_descr = boehm::GC_make_descriptor(&bitmap, bitmap_size);
        boehm::GC_malloc_explicitly_typed(layout.size(), gc_descr) as *mut u8
    }

    #[cfg(feature = "rustgc_internal")]
    fn alloc_conservative(&self, layout: Layout) -> *mut u8 {
        unsafe { boehm::GC_malloc(layout.size()) as *mut u8 }
    }

    #[cfg(feature = "rustgc_internal")]
    unsafe fn alloc_atomic(&self, layout: Layout) -> *mut u8 {
        boehm::GC_malloc_atomic(layout.size()) as *mut u8
    }
}

unsafe impl Allocator for GcAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = unsafe { boehm::GC_malloc(layout.size()) } as *mut u8;
        assert!(!ptr.is_null());
        let ptr = unsafe { NonNull::new_unchecked(ptr) };
        Ok(NonNull::slice_from_raw_parts(ptr, layout.size()))
    }

    unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) {}
}

impl GcAllocator {
    #[cfg(feature = "rustgc_internal")]
    pub fn maybe_optimised_alloc<T>(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let sp = specializer::AllocationSpecializer::new();
        sp.maybe_optimised_alloc::<T>(layout)
    }

    pub fn force_gc() {
        unsafe { boehm::GC_gcollect() }
    }

    pub unsafe fn register_finalizer(
        &self,
        obj: *mut u8,
        finalizer: Option<unsafe extern "C" fn(*mut u8, *mut u8)>,
        client_data: *mut u8,
        old_finalizer: *mut extern "C" fn(*mut u8, *mut u8),
        old_client_data: *mut *mut u8,
    ) {
        boehm::GC_register_finalizer_no_order(
            obj,
            finalizer,
            client_data,
            old_finalizer,
            old_client_data,
        )
    }

    pub fn unregister_finalizer(&self, gcbox: *mut u8) {
        unsafe {
            boehm::GC_register_finalizer(
                gcbox,
                None,
                ::core::ptr::null_mut(),
                ::core::ptr::null_mut(),
                ::core::ptr::null_mut(),
            );
        }
    }

    pub fn get_stats() -> GcStats {
        let mut ps = boehm::ProfileStats::default();
        unsafe {
            boehm::GC_get_prof_stats(
                &mut ps as *mut boehm::ProfileStats,
                core::mem::size_of::<boehm::ProfileStats>(),
            );
        }
        let total_gc_time = unsafe { boehm::GC_get_full_gc_total_time() };

        GcStats {
            total_gc_time,
            num_collections: ps.gc_no,
            total_freed: ps.bytes_reclaimed_since_gc,
            total_alloced: ps.bytes_allocd_since_gc,
        }
    }

    pub fn init() {
        unsafe { boehm::GC_start_performance_measurement() };
    }
}

#[derive(Debug)]
pub struct GcStats {
    total_gc_time: usize, // In milliseconds.
    num_collections: usize,
    total_freed: usize,   // In bytes
    total_alloced: usize, // In bytes
}
