//! `std::process`: `exit`/`abort`/`id`, `Command`/`Child`/`Output`,
//! `ExitStatus`, `ExitCode`. Spawning is fork+execvp in rt.c
//! (`rcc_os_spawn`, with an exec-error pipe so a missing program is a
//! spawn error); `wait_with_output` drains stdout and stderr together with
//! poll(2) so a child writing a lot to both cannot deadlock.

use crate::ffi::{OsStr, OsString};
use crate::fmt;
use crate::fs::File;
use crate::io::{self, Read, Write};
use crate::os::sys;
use crate::path::Path;
use crate::string::String;
use crate::vec::Vec;

pub use crate::rt::Termination;

extern "C" {
    fn rcc_exit(code: i32) -> !;
}

/// Flushes stdout and exits (no destructors run).
pub fn exit(code: i32) -> ! {
    unsafe { rcc_exit(code) }
}

pub fn abort() -> ! {
    unsafe { crate::intrinsics::abort() }
}

pub fn id() -> u32 {
    unsafe { sys::rcc_os_getpid() }
}

// ---------------------------------------------------------------------------
// Stdio

enum StdioKind {
    Inherit,
    Null,
    MakePipe,
    Fd(File),
}

pub struct Stdio(StdioKind);

impl Stdio {
    pub fn piped() -> Stdio {
        Stdio(StdioKind::MakePipe)
    }
    pub fn inherit() -> Stdio {
        Stdio(StdioKind::Inherit)
    }
    pub fn null() -> Stdio {
        Stdio(StdioKind::Null)
    }
    pub fn makes_pipe(&self) -> bool {
        match self.0 {
            StdioKind::MakePipe => true,
            _ => false,
        }
    }
    fn code(&self) -> i32 {
        match &self.0 {
            StdioKind::Inherit => -1,
            StdioKind::Null => -2,
            StdioKind::MakePipe => -3,
            StdioKind::Fd(f) => f.raw(),
        }
    }
}

impl fmt::Debug for Stdio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Stdio { .. }")
    }
}
impl From<File> for Stdio {
    fn from(file: File) -> Stdio {
        Stdio(StdioKind::Fd(file))
    }
}
impl From<ChildStdin> for Stdio {
    fn from(c: ChildStdin) -> Stdio {
        Stdio(StdioKind::Fd(c.inner))
    }
}
impl From<ChildStdout> for Stdio {
    fn from(c: ChildStdout) -> Stdio {
        Stdio(StdioKind::Fd(c.inner))
    }
}
impl From<ChildStderr> for Stdio {
    fn from(c: ChildStderr) -> Stdio {
        Stdio(StdioKind::Fd(c.inner))
    }
}
impl From<io::Stdout> for Stdio {
    fn from(_: io::Stdout) -> Stdio {
        let _ = io::stdout().flush();
        match unsafe { sys::rcc_os_dup(1) } {
            fd if fd >= 0 => Stdio(StdioKind::Fd(File::from_raw(fd))),
            _ => Stdio::inherit(),
        }
    }
}
impl From<io::Stderr> for Stdio {
    fn from(_: io::Stderr) -> Stdio {
        match unsafe { sys::rcc_os_dup(2) } {
            fd if fd >= 0 => Stdio(StdioKind::Fd(File::from_raw(fd))),
            _ => Stdio::inherit(),
        }
    }
}

// ---------------------------------------------------------------------------
// Command

pub struct Command {
    program: OsString,
    args: Vec<OsString>,
    arg0: Option<OsString>,
    env: Vec<(OsString, Option<OsString>)>,
    env_clear: bool,
    cwd: Option<OsString>,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
}

impl Command {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Command {
        Command {
            program: program.as_ref().to_os_string(),
            args: Vec::new(),
            arg0: None,
            env: Vec::new(),
            env_clear: false,
            cwd: None,
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.args.push(arg.as_ref().to_os_string());
        self
    }
    pub fn args<I, S>(&mut self, args: I) -> &mut Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for a in args {
            self.args.push(a.as_ref().to_os_string());
        }
        self
    }
    fn set_env(&mut self, key: OsString, val: Option<OsString>) {
        let mut i = 0;
        while i < self.env.len() {
            if self.env[i].0 == key {
                self.env[i].1 = val;
                return;
            }
            i += 1;
        }
        self.env.push((key, val));
    }
    pub fn env<K: AsRef<OsStr>, V: AsRef<OsStr>>(&mut self, key: K, val: V) -> &mut Command {
        self.set_env(key.as_ref().to_os_string(), Some(val.as_ref().to_os_string()));
        self
    }
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Command
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (k, v) in vars {
            self.set_env(k.as_ref().to_os_string(), Some(v.as_ref().to_os_string()));
        }
        self
    }
    pub fn env_remove<K: AsRef<OsStr>>(&mut self, key: K) -> &mut Command {
        if self.env_clear {
            // nothing inherited: just forget a value set earlier
            let k = key.as_ref().to_os_string();
            self.env.retain(|(ek, _)| *ek != k);
        } else {
            self.set_env(key.as_ref().to_os_string(), None);
        }
        self
    }
    pub fn env_clear(&mut self) -> &mut Command {
        self.env_clear = true;
        self.env.clear();
        self
    }
    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Command {
        self.cwd = Some(dir.as_ref().as_os_str().to_os_string());
        self
    }
    pub fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.stdin = Some(cfg.into());
        self
    }
    pub fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.stdout = Some(cfg.into());
        self
    }
    pub fn stderr<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.stderr = Some(cfg.into());
        self
    }
    pub(crate) fn set_arg0(&mut self, arg: OsString) {
        self.arg0 = Some(arg);
    }
    pub fn get_program(&self) -> &OsStr {
        &self.program
    }
    pub fn get_args(&self) -> CommandArgs<'_> {
        CommandArgs { inner: self.args.iter() }
    }
    pub fn get_envs(&self) -> CommandEnvs<'_> {
        CommandEnvs { inner: self.env.iter() }
    }
    pub fn get_current_dir(&self) -> Option<&Path> {
        match &self.cwd {
            Some(d) => Some(Path::new(d)),
            None => None,
        }
    }

    fn env_block(&self) -> Option<(Vec<u8>, i64)> {
        if !self.env_clear && self.env.is_empty() {
            return None;
        }
        let mut vars: Vec<(OsString, OsString)> = if self.env_clear { Vec::new() } else { crate::env::vars_os().collect() };
        for (k, v) in &self.env {
            let mut i = 0;
            while i < vars.len() {
                if vars[i].0 == *k {
                    vars.remove(i);
                } else {
                    i += 1;
                }
            }
            if let Some(v) = v {
                vars.push((k.clone(), v.clone()));
            }
        }
        let mut block = Vec::new();
        for (k, v) in &vars {
            block.extend_from_slice(k.as_vec());
            block.push(b'=');
            block.extend_from_slice(v.as_vec());
            block.push(0);
        }
        Some((block, vars.len() as i64))
    }

    fn spawn_inner(&mut self, default: i32, needs_stdin: bool) -> io::Result<Child> {
        fn nul_err() -> io::Error {
            io::Error::new_const(io::ErrorKind::InvalidInput, "nul byte found in provided data")
        }
        let mut argbuf: Vec<u8> = Vec::new();
        let first = match &self.arg0 {
            Some(a) => a,
            None => &self.program,
        };
        if first.as_vec().contains(&0) || self.program.as_vec().contains(&0) {
            return Err(nul_err());
        }
        argbuf.extend_from_slice(first.as_vec());
        argbuf.push(0);
        for a in &self.args {
            if a.as_vec().contains(&0) {
                return Err(nul_err());
            }
            argbuf.extend_from_slice(a.as_vec());
            argbuf.push(0);
        }
        // the program path for execvp; argv[0] (arg0) is separate
        let mut prog = self.program.as_vec().clone();
        prog.push(0);
        let argc = self.args.len() + 1;
        let env = self.env_block();
        let mut cwd_buf: Vec<u8> = Vec::new();
        let cwd_ptr = match &self.cwd {
            Some(d) => {
                if d.as_vec().contains(&0) {
                    return Err(nul_err());
                }
                cwd_buf.extend_from_slice(d.as_vec());
                cwd_buf.push(0);
                cwd_buf.as_ptr()
            }
            None => crate::ptr::null(),
        };
        let stdin_code = match &self.stdin {
            Some(s) => s.code(),
            None => {
                if needs_stdin { default } else { -2 }
            }
        };
        let stdout_code = match &self.stdout {
            Some(s) => s.code(),
            None => default,
        };
        let stderr_code = match &self.stderr {
            Some(s) => s.code(),
            None => default,
        };
        let stdio = [stdin_code, stdout_code, stderr_code];
        let mut pid = 0i32;
        let mut pipes = [-1i32; 3];
        let (env_ptr, envc) = match &env {
            Some((b, n)) => (b.as_ptr(), *n),
            None => (crate::ptr::null(), -1),
        };
        let r = unsafe {
            sys::rcc_os_spawn(prog.as_ptr(), argbuf.as_ptr(), argc, env_ptr, envc, cwd_ptr, stdio.as_ptr(), &mut pid, pipes.as_mut_ptr())
        };
        if r != 0 {
            return Err(io::Error::from_raw_os_error(r));
        }
        Ok(Child {
            pid,
            status: None,
            stdin: if pipes[0] >= 0 { Some(ChildStdin { inner: File::from_raw(pipes[0]) }) } else { None },
            stdout: if pipes[1] >= 0 { Some(ChildStdout { inner: File::from_raw(pipes[1]) }) } else { None },
            stderr: if pipes[2] >= 0 { Some(ChildStderr { inner: File::from_raw(pipes[2]) }) } else { None },
        })
    }

    pub fn spawn(&mut self) -> io::Result<Child> {
        self.spawn_inner(-1, true)
    }
    pub fn output(&mut self) -> io::Result<Output> {
        let child = self.spawn_inner(-3, false)?;
        child.wait_with_output()
    }
    pub fn status(&mut self) -> io::Result<ExitStatus> {
        let mut child = self.spawn_inner(-1, true)?;
        child.wait()
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(cwd) = &self.cwd {
            write!(f, "cd {:?} && ", cwd)?;
        }
        let mut envs: Vec<&(OsString, Option<OsString>)> = self.env.iter().collect();
        envs.sort_by(|a, b| a.0.cmp(&b.0));
        if self.env_clear {
            f.write_str("env -i ")?;
        } else {
            let mut any_removed = false;
            for (k, v) in &envs {
                if v.is_none() {
                    if !any_removed {
                        f.write_str("env ")?;
                        any_removed = true;
                    }
                    write!(f, "-u {} ", k.to_string_lossy())?;
                }
            }
        }
        for (k, v) in &envs {
            if let Some(v) = v {
                write!(f, "{}={:?} ", k.to_string_lossy(), v)?;
            }
        }
        let first = match &self.arg0 {
            Some(a) => a,
            None => &self.program,
        };
        if *first != self.program {
            write!(f, "[{:?}] ", self.program)?;
        }
        write!(f, "{:?}", first)?;
        for a in &self.args {
            write!(f, " {:?}", a)?;
        }
        Ok(())
    }
}

pub struct CommandArgs<'a> {
    inner: crate::slice::Iter<'a, OsString>,
}
impl<'a> Iterator for CommandArgs<'a> {
    type Item = &'a OsStr;
    fn next(&mut self) -> Option<&'a OsStr> {
        match self.inner.next() {
            Some(s) => Some(s.as_os_str()),
            None => None,
        }
    }
}
impl<'a> ExactSizeIterator for CommandArgs<'a> {}

pub struct CommandEnvs<'a> {
    inner: crate::slice::Iter<'a, (OsString, Option<OsString>)>,
}
impl<'a> Iterator for CommandEnvs<'a> {
    type Item = (&'a OsStr, Option<&'a OsStr>);
    fn next(&mut self) -> Option<(&'a OsStr, Option<&'a OsStr>)> {
        match self.inner.next() {
            Some((k, v)) => Some((
                k.as_os_str(),
                match v {
                    Some(v) => Some(v.as_os_str()),
                    None => None,
                },
            )),
            None => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Child

pub struct ChildStdin {
    inner: File,
}
pub struct ChildStdout {
    inner: File,
}
pub struct ChildStderr {
    inner: File,
}

impl ChildStdin {
    pub(crate) fn raw(&self) -> i32 {
        self.inner.raw()
    }
}
impl ChildStdout {
    pub(crate) fn raw(&self) -> i32 {
        self.inner.raw()
    }
}
impl ChildStderr {
    pub(crate) fn raw(&self) -> i32 {
        self.inner.raw()
    }
}

impl Write for ChildStdin {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl<'a> Write for &'a ChildStdin {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&self.inner).write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Read for ChildStdout {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
impl Read for ChildStderr {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
impl fmt::Debug for ChildStdin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ChildStdin { .. }")
    }
}
impl fmt::Debug for ChildStdout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ChildStdout { .. }")
    }
}
impl fmt::Debug for ChildStderr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ChildStderr { .. }")
    }
}

pub struct Child {
    pid: i32,
    status: Option<ExitStatus>,
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    pub stderr: Option<ChildStderr>,
}

impl fmt::Debug for Child {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Child")
            .field("stdin", &self.stdin)
            .field("stdout", &self.stdout)
            .field("stderr", &self.stderr)
            .finish_non_exhaustive()
    }
}

/// Read from one pipe until EOF.
fn read_all(f: &mut File, out: &mut Vec<u8>) -> io::Result<()> {
    f.read_to_end(out)?;
    Ok(())
}

/// Drain two pipes concurrently (poll) until both reach EOF.
fn read2(mut a: Option<File>, out_a: &mut Vec<u8>, mut b: Option<File>, out_b: &mut Vec<u8>) -> io::Result<()> {
    let mut chunk = crate::vec::from_elem(0u8, 64 * 1024);
    loop {
        let fa = match &a {
            Some(f) => f.raw(),
            None => -1,
        };
        let fb = match &b {
            Some(f) => f.raw(),
            None => -1,
        };
        if fa < 0 && fb < 0 {
            return Ok(());
        }
        if fa < 0 {
            if let Some(f) = &mut b {
                read_all(f, out_b)?;
            }
            return Ok(());
        }
        if fb < 0 {
            if let Some(f) = &mut a {
                read_all(f, out_a)?;
            }
            return Ok(());
        }
        let m = unsafe { sys::rcc_os_poll2(fa, fb) };
        if m < 0 {
            return Err(io::Error::from_raw_os_error(-m));
        }
        if m & 1 != 0 {
            let n = sys::cvt(unsafe { sys::rcc_os_read(fa, chunk.as_mut_ptr(), chunk.len()) })? as usize;
            if n == 0 {
                a = None;
            } else {
                out_a.extend_from_slice(&chunk[..n]);
            }
        }
        if m & 2 != 0 {
            let n = sys::cvt(unsafe { sys::rcc_os_read(fb, chunk.as_mut_ptr(), chunk.len()) })? as usize;
            if n == 0 {
                b = None;
            } else {
                out_b.extend_from_slice(&chunk[..n]);
            }
        }
    }
}

impl Child {
    pub fn kill(&mut self) -> io::Result<()> {
        if self.status.is_some() {
            return Ok(());
        }
        let r = unsafe { sys::rcc_os_kill(self.pid, sys::rcc_os_sigkill()) };
        if r < 0 { Err(io::Error::from_raw_os_error(-r)) } else { Ok(()) }
    }
    pub fn id(&self) -> u32 {
        self.pid as u32
    }
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        drop(self.stdin.take());
        if let Some(s) = self.status {
            return Ok(s);
        }
        let mut st = 0i32;
        let r = unsafe { sys::rcc_os_wait(self.pid, &mut st, 0) };
        if r < 0 {
            return Err(io::Error::from_raw_os_error(-r));
        }
        let s = ExitStatus(st);
        self.status = Some(s);
        Ok(s)
    }
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if let Some(s) = self.status {
            return Ok(Some(s));
        }
        let mut st = 0i32;
        let r = unsafe { sys::rcc_os_wait(self.pid, &mut st, 1) };
        if r < 0 {
            return Err(io::Error::from_raw_os_error(-r));
        }
        if r == 0 {
            return Ok(None);
        }
        let s = ExitStatus(st);
        self.status = Some(s);
        Ok(Some(s))
    }
    pub fn wait_with_output(mut self) -> io::Result<Output> {
        drop(self.stdin.take());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let out = match self.stdout.take() {
            Some(c) => Some(c.inner),
            None => None,
        };
        let err = match self.stderr.take() {
            Some(c) => Some(c.inner),
            None => None,
        };
        read2(out, &mut stdout, err, &mut stderr)?;
        let status = self.wait()?;
        Ok(Output { status, stdout, stderr })
    }
}

// ---------------------------------------------------------------------------
// Output, ExitStatus, ExitCode

#[derive(PartialEq, Eq, Clone)]
pub struct Output {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

struct BytesOrStr<'a>(&'a [u8]);
impl<'a> fmt::Debug for BytesOrStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match crate::str::from_utf8(self.0) {
            Ok(s) => fmt::Debug::fmt(s, f),
            Err(_) => fmt::Debug::fmt(self.0, f),
        }
    }
}

impl fmt::Debug for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Output")
            .field("status", &self.status)
            .field("stdout", &BytesOrStr(&self.stdout))
            .field("stderr", &BytesOrStr(&self.stderr))
            .finish()
    }
}

/// Raw wait(2) status.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct ExitStatus(i32);

impl ExitStatus {
    fn decode(&self) -> (i32, i32, i32) {
        let mut val = 0i32;
        let mut core = 0i32;
        let k = unsafe { sys::rcc_os_wstatus(self.0, &mut val, &mut core) };
        (k, val, core)
    }
    pub fn success(&self) -> bool {
        self.code() == Some(0)
    }
    pub fn code(&self) -> Option<i32> {
        let (k, v, _) = self.decode();
        if k == 0 { Some(v) } else { None }
    }
    pub fn exit_ok(&self) -> Result<(), ExitStatusError> {
        if self.success() { Ok(()) } else { Err(ExitStatusError(*self)) }
    }
    pub(crate) fn from_raw(raw: i32) -> ExitStatus {
        ExitStatus(raw)
    }
    pub(crate) fn into_raw(self) -> i32 {
        self.0
    }
    pub(crate) fn signal(&self) -> Option<i32> {
        let (k, v, _) = self.decode();
        if k == 1 { Some(v) } else { None }
    }
    pub(crate) fn core_dumped(&self) -> bool {
        let (k, _, c) = self.decode();
        k == 1 && c != 0
    }
    pub(crate) fn stopped_signal(&self) -> Option<i32> {
        let (k, v, _) = self.decode();
        if k == 2 { Some(v) } else { None }
    }
    pub(crate) fn continued(&self) -> bool {
        self.decode().0 == 3
    }
}

impl Default for ExitStatus {
    fn default() -> ExitStatus {
        ExitStatus(0)
    }
}

impl fmt::Debug for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExitStatus(unix_wait_status({}))", self.0)
    }
}

fn signal_name(sig: i32) -> &'static str {
    let p = unsafe { sys::rcc_os_signame(sig) };
    let n = sys::c_strlen(p);
    unsafe { crate::str::from_utf8_unchecked(crate::slice::from_raw_parts(p, n)) }
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (k, v, core) = self.decode();
        if k == 0 {
            write!(f, "exit status: {}", v)
        } else if k == 1 {
            let name = signal_name(v);
            let name = if name.is_empty() { String::new() } else { crate::fmt::format(format_args!(" ({})", name)) };
            if core != 0 {
                write!(f, "signal: {}{} (core dumped)", v, name)
            } else {
                write!(f, "signal: {}{}", v, name)
            }
        } else if k == 2 {
            let name = signal_name(v);
            let name = if name.is_empty() { String::new() } else { crate::fmt::format(format_args!(" ({})", name)) };
            write!(f, "stopped (not terminated) by signal: {}{}", v, name)
        } else if k == 3 {
            f.write_str("continued (WIFCONTINUED)")
        } else {
            write!(f, "unrecognised wait status: {} {:#x}", self.0, self.0)
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ExitStatusError(ExitStatus);

impl ExitStatusError {
    pub fn code(&self) -> Option<i32> {
        self.0.code()
    }
    pub fn into_status(&self) -> ExitStatus {
        self.0
    }
}
impl fmt::Display for ExitStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "process exited unsuccessfully: {}", self.0)
    }
}
impl crate::error::Error for ExitStatusError {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ExitCode(u8);

impl ExitCode {
    pub const SUCCESS: ExitCode = ExitCode(0);
    pub const FAILURE: ExitCode = ExitCode(1);

    pub fn exit_process(self) -> ! {
        exit(self.0 as i32)
    }
}

impl From<u8> for ExitCode {
    fn from(code: u8) -> ExitCode {
        ExitCode(code)
    }
}

impl Default for ExitCode {
    fn default() -> ExitCode {
        ExitCode::SUCCESS
    }
}

impl fmt::Debug for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExitCode(unix_exit_status({}))", self.0)
    }
}

impl Termination for ExitCode {
    fn report(self) -> i32 {
        self.0 as i32
    }
}
