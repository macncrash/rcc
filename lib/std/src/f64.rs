//! `std::f64` module: constants.
pub mod consts {
    pub use crate::num::float::consts_f64::*;
}
pub const INFINITY: f64 = 1.0 / 0.0;
pub const NEG_INFINITY: f64 = -1.0 / 0.0;
pub const NAN: f64 = 0.0 / 0.0;
pub const MAX: f64 = 1.7976931348623157e308;
pub const MIN: f64 = -1.7976931348623157e308;
pub const EPSILON: f64 = 2.220446049250313e-16;
