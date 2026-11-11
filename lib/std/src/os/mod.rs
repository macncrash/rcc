//! OS-specific extensions: `std::os::unix::{fs, ffi, process, io}` and
//! `std::os::fd`. Also home of the private `sys` module (the runtime
//! declarations used by io/fs/env/process/time).

pub(crate) mod sys;
pub mod fd;
pub mod unix;

pub mod raw {
    pub type c_char = i8;
    pub type c_int = i32;
    pub type c_uint = u32;
    pub type c_long = i64;
    pub type c_ulong = u64;
    pub type c_void = u8;
}
