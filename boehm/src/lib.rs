#![feature(allocator_api)]
#![feature(nonnull_slice_from_raw_parts)]

pub mod allocator;
mod ffi;

use std::mem::{needs_drop, ManuallyDrop};

use ffi::ProfileStats;

pub fn register_finalizer<T>(gcbox: *mut T) {
    if !needs_drop::<T>() {
        return;
    }

    unsafe extern "C" fn fshim<T>(obj: *mut u8, _meta: *mut u8) {
        ManuallyDrop::drop(&mut *(obj as *mut ManuallyDrop<T>));
    }

    unsafe {
        ffi::gc_register_finalizer(
            gcbox as *mut u8,
            Some(fshim::<T>),
            ::std::ptr::null_mut(),
            ::std::ptr::null_mut(),
            ::std::ptr::null_mut(),
        );
    }
}

pub fn unregister_finalizer(gcbox: *mut u8) {
    unsafe {
        ffi::gc_register_finalizer(
            gcbox,
            None,
            ::std::ptr::null_mut(),
            ::std::ptr::null_mut(),
            ::std::ptr::null_mut(),
        );
    }
}

pub struct BoehmStats {
    pub total_gc_time: usize, // In milliseconds.
    pub num_collections: usize,
    pub total_freed: usize,   // In bytes
    pub total_alloced: usize, // In bytes
}

impl BoehmStats {
    pub fn gen() -> Self {
        let mut ps = ProfileStats::default();
        unsafe {
            ffi::gc_get_prof_stats(
                &mut ps as *mut ProfileStats,
                std::mem::size_of::<ProfileStats>(),
            );
        }
        let total_gc_time = unsafe { ffi::gc_get_full_gc_total_time() };

        BoehmStats {
            total_gc_time,
            num_collections: ps.gc_no,
            total_freed: ps.bytes_reclaimed_since_gc,
            total_alloced: ps.bytes_allocd_since_gc,
        }
    }
}

pub fn init() {
    unsafe { ffi::gc_start_performance_measurement() };
}
