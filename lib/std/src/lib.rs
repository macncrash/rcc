//! rcc's standard library (also reachable as `core` and `alloc`).
//!
//! This is NOT the real libstd. It is plain Rust compiled from source by rcc
//! together with every program, using only:
//!   - `#[lang = "..."]` to mark the items the compiler knows about,
//!   - `extern "rcc-intrinsic"` functions (see intrinsics.rs),
//!   - `extern "C"` functions implemented in rt/rt.c (all named `rcc_*`),
//!     plus a few libm functions with their exact C prototypes.
//!
//! Layout: one module per std module; see docs/ARCHITECTURE.md ("How to add
//! a std API"). Language-level core (owned by the compiler core): marker,
//! ops, cmp, clone, default, convert, option, result, intrinsics, mem, ptr,
//! alloc, boxed, num, char, slice, str, string, vec, iter, fmt, io (printing
//! only), rt, process, prelude.

pub mod intrinsics;
pub mod marker;
pub mod ops;
pub mod clone;
pub mod cmp;
pub mod default;
pub mod convert;
pub mod option;
pub mod result;
pub mod mem;
pub mod ptr;
pub mod alloc;
pub mod boxed;
pub mod num;
pub mod char;
pub mod bool;
pub mod iter;
pub mod slice;
pub mod str;
pub mod string;
pub mod vec;
pub mod fmt;
pub mod io;
pub mod rt;
pub mod process;
pub mod hash;
pub mod borrow;
pub mod any;
pub mod hint;
pub mod array;
pub mod f32;
pub mod f64;
pub mod cell;
pub mod rc;
pub mod prelude;

pub mod primitive_docs {}
pub mod collections;
pub mod sync;
pub mod thread;
pub mod panic;
pub mod error;
pub mod ffi;
pub mod os;
pub mod path;
pub mod fs;
pub mod env;
pub mod time;
