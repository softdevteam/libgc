# libgc

libgc is a garbage collector for Rust. It works by providing a garbage-collected
`Gc<T>` smart pointer in the style of `Rc<T>`.

# Structure

There are two repositories which make up the gc infrastructure:

* **libgc** the main library which provides the `Gc<T>` smart pointer and its
      API.
* **rustgc** a fork of rustc with GC-aware optimisations. This can be used to
      compile user programs which use `libgc`, giving them better GC
      performance. Use of rustgc is not mandated, but it enables further
      optimisations for programs which use `libgc`.

This seperation between libgc and rustgc exists so that a stripped-down form of
garbage collection can be used without compiler support. 
