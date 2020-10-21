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

#[cfg(feature = "gc_stats")]
impl From<boehm::BoehmStats> for GcStats {
    fn from(item: boehm::BoehmStats) -> Self {
        GcStats {
            total_gc_time: item.total_gc_time,
            num_collections: item.num_collections,
            finalizers_registered: NUM_REGISTERED_FINALIZERS.load(Ordering::Relaxed),
            total_freed: item.total_freed,
            total_alloced: item.total_alloced,
        }
    }
}

#[cfg(feature = "gc_stats")]
pub fn get_stats() -> GcStats {
    GcStats::from(boehm::BoehmStats::gen())
}
