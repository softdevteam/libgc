#![feature(core_intrinsics)]
#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(raw_vec_internals)]
#![feature(const_fn)]
#![feature(coerce_unsized)]
#![feature(unsize)]
#![feature(maybe_uninit_ref)]
#[cfg(not(all(target_pointer_width = "64", target_arch = "x86_64")))]
compile_error!("Requires x86_64 with 64 bit pointer width.");

pub mod gc;

mod allocator;
mod boehm;

pub use gc::Gc;

use crate::allocator::{GcAllocator, GlobalAllocator};

#[global_allocator]
static ALLOCATOR: GlobalAllocator = GlobalAllocator;
static mut GC_ALLOCATOR: GcAllocator = GcAllocator;
