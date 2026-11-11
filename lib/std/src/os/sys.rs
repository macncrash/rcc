//! Declarations of the `rcc_os_*` runtime functions (rt/rt.c, block
//! "rcc_os_*") plus small helpers shared by io/fs/env/process/time.
//! All libc structs stay on the C side; only scalars and byte buffers
//! cross this boundary. Functions return `-errno` on failure unless
//! documented otherwise in rt.c.

use crate::io;
use crate::vec::Vec;

extern "C" {
    pub fn rcc_os_error_kind(e: i32) -> i32;
    pub fn rcc_os_strerror(e: i32, buf: *mut u8, cap: usize) -> usize;

    pub fn rcc_os_read(fd: i32, buf: *mut u8, len: usize) -> i64;
    pub fn rcc_os_write(fd: i32, buf: *const u8, len: usize) -> i64;
    pub fn rcc_os_close(fd: i32) -> i32;
    pub fn rcc_os_dup(fd: i32) -> i32;
    pub fn rcc_os_isatty(fd: i32) -> i32;

    pub fn rcc_os_stdout_write(buf: *const u8, len: usize) -> i32;
    pub fn rcc_os_stdout_flush() -> i32;
    pub fn rcc_os_stdin_fill(ptr: *mut *const u8) -> i64;
    pub fn rcc_os_stdin_consume(n: usize);
    pub fn rcc_os_stdin_read(buf: *mut u8, len: usize) -> i64;

    pub fn rcc_os_open(path: *const u8, len: usize, flags: i32, custom: i32, mode: u32) -> i32;
    pub fn rcc_os_lseek(fd: i32, off: i64, whence: i32) -> i64;
    pub fn rcc_os_ftruncate(fd: i32, size: u64) -> i32;
    pub fn rcc_os_fsync(fd: i32, data_only: i32) -> i32;
    pub fn rcc_os_fchmod(fd: i32, mode: u32) -> i32;
    pub fn rcc_os_stat(path: *const u8, len: usize, follow: i32, out: *mut u64) -> i32;
    pub fn rcc_os_fstat(fd: i32, out: *mut u64) -> i32;

    pub fn rcc_os_unlink(path: *const u8, len: usize) -> i32;
    pub fn rcc_os_rmdir(path: *const u8, len: usize) -> i32;
    pub fn rcc_os_chdir(path: *const u8, len: usize) -> i32;
    pub fn rcc_os_mkdir(path: *const u8, len: usize, mode: u32) -> i32;
    pub fn rcc_os_chmod(path: *const u8, len: usize, mode: u32) -> i32;
    pub fn rcc_os_path2(op: i32, a: *const u8, alen: usize, b: *const u8, blen: usize) -> i32;
    pub fn rcc_os_path_query(op: i32, path: *const u8, len: usize, out: *mut *mut u8) -> i64;
    pub fn rcc_os_getcwd(out: *mut *mut u8) -> i64;
    pub fn rcc_os_current_exe(out: *mut *mut u8) -> i64;

    pub fn rcc_os_opendir(path: *const u8, len: usize, err: *mut i32) -> usize;
    pub fn rcc_os_readdir(dir: usize, name: *mut *const u8, len: *mut usize, ty: *mut i32, ino: *mut u64) -> i32;
    pub fn rcc_os_closedir(dir: usize);

    pub fn rcc_os_getenv(key: *const u8, len: usize, out: *mut *mut u8) -> i64;
    pub fn rcc_os_setenv(key: *const u8, klen: usize, val: *const u8, vlen: usize) -> i32;
    pub fn rcc_os_unsetenv(key: *const u8, klen: usize) -> i32;
    pub fn rcc_os_environ(out: *mut *mut u8) -> usize;
    pub fn rcc_os_getpid() -> u32;

    pub fn rcc_os_clock(which: i32, secs: *mut i64, nanos: *mut i64);
    pub fn rcc_os_sleep(secs: u64, nanos: u32);

    pub fn rcc_os_spawn(
        prog: *const u8,
        args: *const u8,
        argc: usize,
        envs: *const u8,
        envc: i64,
        cwd: *const u8,
        stdio: *const i32,
        pid: *mut i32,
        pipes: *mut i32,
    ) -> i32;
    pub fn rcc_os_wait(pid: i32, status: *mut i32, nohang: i32) -> i32;
    pub fn rcc_os_kill(pid: i32, sig: i32) -> i32;
    pub fn rcc_os_sigkill() -> i32;
    pub fn rcc_os_wstatus(st: i32, val: *mut i32, core: *mut i32) -> i32;
    pub fn rcc_os_signame(sig: i32) -> *const u8;
    pub fn rcc_os_poll2(a: i32, b: i32) -> i32;
    pub fn rcc_os_pipe(fds: *mut i32) -> i32;

    fn rcc_dealloc(p: *mut u8);
}

/// `Ok(r)` for `r >= 0`, else the OS error `-r`.
pub fn cvt(r: i64) -> io::Result<i64> {
    if r < 0 { Err(io::Error::from_raw_os_error((-r) as i32)) } else { Ok(r) }
}

pub fn cvt32(r: i32) -> io::Result<i32> {
    if r < 0 { Err(io::Error::from_raw_os_error(-r)) } else { Ok(r) }
}

/// Take ownership of a malloc'd buffer of `n` bytes returned by rt.c.
pub fn take_c_buf(p: *mut u8, n: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(n);
    unsafe {
        crate::intrinsics::copy_nonoverlapping(p as *const u8, v.as_mut_ptr(), n);
        v.set_len(n);
        rcc_dealloc(p);
    }
    v
}

/// Result of a C function that returns a malloc'd buffer and its length.
pub fn c_buf_result(n: i64, p: *mut u8) -> io::Result<Vec<u8>> {
    if n < 0 { Err(io::Error::from_raw_os_error((-n) as i32)) } else { Ok(take_c_buf(p, n as usize)) }
}

/// Length of a NUL-terminated C string.
pub fn c_strlen(p: *const u8) -> usize {
    let mut n = 0usize;
    unsafe {
        while *crate::intrinsics::ptr_offset(p, n as isize) != 0 {
            n += 1;
        }
    }
    n
}
