//! `std::f32` module: constants.
pub mod consts {
    pub use crate::num::float::consts_f32::*;
}
pub const INFINITY: f32 = 1.0 / 0.0;
pub const NEG_INFINITY: f32 = -1.0 / 0.0;
pub const NAN: f32 = 0.0 / 0.0;
pub const MAX: f32 = 3.40282347e38;
pub const MIN: f32 = -3.40282347e38;
pub const EPSILON: f32 = 1.1920929e-7;
