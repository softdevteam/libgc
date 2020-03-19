use libc::size_t;

use std::ffi::c_void;

#[link(name = "gc")]
extern "C" {
    pub fn GC_gcollect();

    pub fn GC_malloc(nbytes: size_t) -> *mut c_void;

    pub fn GC_malloc_uncollectable(nbytes: size_t) -> *mut c_void;

    pub fn GC_realloc(old: *mut c_void, new_size: size_t) -> *mut c_void;

    pub fn GC_free(dead: *mut c_void);

    pub fn GC_register_finalizer(
        ptr: *mut c_void,
        finalizer: unsafe extern "C" fn(*mut c_void, *mut c_void),
        client_data: *mut c_void,
        old_finalizer: *mut extern "C" fn(*mut c_void, *mut c_void),
        old_client_data: *mut *mut c_void,
    );
}
