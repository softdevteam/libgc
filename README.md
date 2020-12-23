# libgc

libgc is a garbage collector for Rust. It works by providing a garbage-collected
`Gc<T>` smart pointer in the style of `Rc<T>`.

# Structure

There are three repositories which make up the gc infrastructure:
    - **libgc** the main library which provides the `Gc<T>` smart pointer and its
      API.
    - **libgc_internal** contains the gc allocation and collector logic. This is
      collector specific, and can be conditionally compiled to support different
      implementations. At the moment, it only supports a single collector
      implementation: the Boehm-Demers-Weiser GC. Users should never interact
      directly with this crate. Instead, any relevant APIs are re-exported
      through libgc.
    - **rustgc** a fork of rustc with GC-aware optimisations. This can be used to
      compile user programs which use `libgc`, giving them better GC
      performance. Use of rustgc is not mandated, but it enables further
      optimisations for programs which use `libgc`.

This seperation between libgc and rustgc exists so that a stripped-down form of
garbage collection can be used without compiler support. The further split
between libgc and libgc_core exists to make linkage easier when the rustgc
compiler is used.

rustgc needs access to the GC's `Allocator` implementation. This exists in the
libgc_internal crate so that it can be linked to the target binary either as
part of libgc, or as part of the rust standard library (if compiled with
rustgc). libgc contains code which would not compile if it was packaged as part
of rustgc. To prevent duplication, the libgc_interal crate will link correctly
as either a standard cargo crate, or as part of the rust core library.
