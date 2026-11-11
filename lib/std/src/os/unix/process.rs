//! `std::os::unix::process::{ExitStatusExt, CommandExt}`.

use crate::ffi::OsStr;
use crate::process::{Command, ExitStatus};

pub trait ExitStatusExt {
    fn from_raw(raw: i32) -> Self;
    fn signal(&self) -> Option<i32>;
    fn core_dumped(&self) -> bool;
    fn stopped_signal(&self) -> Option<i32>;
    fn continued(&self) -> bool;
    fn into_raw(self) -> i32;
}

impl ExitStatusExt for ExitStatus {
    fn from_raw(raw: i32) -> ExitStatus {
        ExitStatus::from_raw(raw)
    }
    fn signal(&self) -> Option<i32> {
        ExitStatus::signal(self)
    }
    fn core_dumped(&self) -> bool {
        ExitStatus::core_dumped(self)
    }
    fn stopped_signal(&self) -> Option<i32> {
        ExitStatus::stopped_signal(self)
    }
    fn continued(&self) -> bool {
        ExitStatus::continued(self)
    }
    fn into_raw(self) -> i32 {
        ExitStatus::into_raw(self)
    }
}

pub trait CommandExt {
    fn arg0<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command;
}

impl CommandExt for Command {
    fn arg0<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.set_arg0(arg.as_ref().to_os_string());
        self
    }
}
