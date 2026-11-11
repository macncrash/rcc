pub mod b;
use crate::late_export;
pub fn name() -> &'static str {
    "a"
}
pub fn uses_exported() -> String {
    format!("{} via use, twice {}", late_export!(), twice!(4))
}
