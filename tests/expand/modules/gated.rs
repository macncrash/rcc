#![cfg(feature = "never")]
pub fn broken() -> i32 { undefined_function() }
