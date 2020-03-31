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

mod boehm;

pub mod allocator;
pub mod gc;

pub use gc::Gc;
pub use crate::allocator::BoehmAllocator;

use crate::allocator::BoehmGcAllocator;

static mut GC_ALLOCATOR: BoehmGcAllocator = BoehmGcAllocator;

