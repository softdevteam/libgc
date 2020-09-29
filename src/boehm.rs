#[cfg(feature = "gc_stats")]
use crate::stats::ProfileStats;

#[cfg(not(feature = "rustc_boehm"))]
pub unsafe fn gc_malloc(size: usize) -> *mut u8 {
    GC_malloc(size) as *mut u8
}

#[cfg(not(feature = "rustc_boehm"))]
pub unsafe fn gc_realloc(old: *mut u8, new_size: usize) -> *mut u8 {
    GC_realloc(old, new_size) as *mut u8
}

#[cfg(not(feature = "rustc_boehm"))]
pub unsafe fn gc_malloc_uncollectable(size: usize) -> *mut u8 {
    GC_malloc_uncollectable(size) as *mut u8
}

#[cfg(not(feature = "rustc_boehm"))]
pub unsafe fn gc_free(dead: *mut u8) {
    GC_free(dead)
}

#[cfg(feature = "rustc_boehm")]
pub unsafe fn gc_register_finalizer(
    obj: *mut u8,
    finalizer: Option<unsafe extern "C" fn(*mut u8, *mut u8)>,
    client_data: *mut u8,
    old_finalizer: *mut extern "C" fn(*mut u8, *mut u8),
    old_client_data: *mut *mut u8,
) {
    std::boehm::gc_register_finalizer(obj, finalizer, client_data, old_finalizer, old_client_data)
}

#[cfg(not(feature = "rustc_boehm"))]
pub unsafe fn gc_register_finalizer(
    obj: *mut u8,
    finalizer: Option<unsafe extern "C" fn(*mut u8, *mut u8)>,
    client_data: *mut u8,
    old_finalizer: *mut extern "C" fn(*mut u8, *mut u8),
    old_client_data: *mut *mut u8,
) {
    GC_register_finalizer(obj, finalizer, client_data, old_finalizer, old_client_data)
}

#[cfg(not(feature = "rustc_boehm"))]
#[cfg(feature = "gc_stats")]
pub unsafe fn gc_start_performance_measurement() {
    GC_start_performance_measurement();
}

#[cfg(not(feature = "rustc_boehm"))]
#[cfg(feature = "gc_stats")]
pub unsafe fn gc_get_full_gc_total_time() -> usize {
    GC_get_full_gc_total_time()
}

#[cfg(not(feature = "rustc_boehm"))]
#[cfg(feature = "gc_stats")]
pub unsafe fn gc_get_prof_stats(prof_stats: *mut ProfileStats, stats_size: usize) -> usize {
    GC_get_prof_stats(prof_stats, stats_size)
}

#[link(name = "gc")]
#[cfg(not(feature = "rustc_boehm"))]
extern "C" {
    fn GC_malloc(nbytes: usize) -> *mut u8;

    fn GC_malloc_uncollectable(nbytes: usize) -> *mut u8;

    fn GC_realloc(old: *mut u8, new_size: usize) -> *mut u8;

    fn GC_free(dead: *mut u8);

    fn GC_register_finalizer(
        ptr: *mut u8,
        finalizer: Option<unsafe extern "C" fn(*mut u8, *mut u8)>,
        client_data: *mut u8,
        old_finalizer: *mut extern "C" fn(*mut u8, *mut u8),
        old_client_data: *mut *mut u8,
    );

    #[cfg(feature = "gc_stats")]
    fn GC_start_performance_measurement();

    #[cfg(feature = "gc_stats")]
    fn GC_get_full_gc_total_time() -> usize;

    #[cfg(feature = "gc_stats")]
    fn GC_get_prof_stats(prof_stats: *mut ProfileStats, stats_size: usize) -> usize;
}
