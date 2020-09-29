use crate::boehm;
use std::sync::atomic::{AtomicUsize, Ordering};

pub static NUM_REGISTERED_FINALIZERS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
pub struct GcStats {
    total_gc_time: usize, // In milliseconds.
    num_collections: usize,
    finalizers_registered: usize,
    total_freed: usize,   // In bytes
    total_alloced: usize, // In bytes
}

impl GcStats {
    fn gen() -> Self {
        let mut ps = ProfileStats::default();
        unsafe {
            boehm::gc_get_prof_stats(
                &mut ps as *mut ProfileStats,
                std::mem::size_of::<ProfileStats>(),
            );
        }

        let total_gc_time = unsafe { boehm::gc_get_full_gc_total_time() };

        GcStats {
            total_gc_time,
            num_collections: ps.gc_no,
            finalizers_registered: NUM_REGISTERED_FINALIZERS.load(Ordering::Relaxed),
            total_freed: ps.bytes_reclaimed_since_gc,
            total_alloced: ps.bytes_allocd_since_gc,
        }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct ProfileStats {
    /// Heap size in bytes (including area unmapped to OS).
    heapsize_full: usize,
    /// Total bytes contained in free and unmapped blocks.
    free_bytes_full: usize,
    /// Amount of memory unmapped to OS.
    unmapped_bytes: usize,
    /// Number of bytes allocated since the recent collection.
    bytes_allocd_since_gc: usize,
    /// Number of bytes allocated before the recent collection.
    /// The value may wrap.
    allocd_bytes_before_gc: usize,
    /// Number of bytes not considered candidates for garbage collection.
    non_gc_bytes: usize,
    /// Garbage collection cycle number.
    /// The value may wrap.
    gc_no: usize,
    /// Number of marker threads (excluding the initiating one).
    markers_m1: usize,
    /// Approximate number of reclaimed bytes after recent collection.
    bytes_reclaimed_since_gc: usize,
    /// Approximate number of bytes reclaimed before the recent collection.
    /// The value may wrap.
    reclaimed_bytes_before_gc: usize,
    /// Number of bytes freed explicitly since the recent GC.
    expl_freed_bytes_since_gc: usize,
}

#[cfg(feature = "gc_stats")]
pub fn get_stats() -> GcStats {
    GcStats::gen()
}
