// Re-exports with `pub use`, renaming re-exports, glob re-exports, enum variants.
mod inner {
    pub mod engine {
        pub struct Engine {
            pub hp: u32,
        }
        pub fn start(e: &Engine) -> String {
            format!("engine {} hp", e.hp)
        }
        pub enum Mode {
            Eco,
            Sport(u8),
        }
    }
    pub mod util {
        pub fn double(x: u32) -> u32 { x * 2 }
        pub fn triple(x: u32) -> u32 { x * 3 }
    }
    pub use self::engine::{start as ignite, Engine};
    pub use self::engine::Mode::{self, Eco, Sport};
    pub use self::util::*;
}

pub mod api {
    pub use crate::inner::Engine as Motor;
    pub use crate::inner::{double, ignite};
    pub mod prelude {
        pub use super::Motor;
        pub use crate::inner::triple;
    }
}

use api::prelude::*;
use inner::Mode;

fn describe(m: &Mode) -> String {
    match m {
        inner::Eco => "eco".to_string(),
        inner::Sport(n) => format!("sport level {}", n),
    }
}

fn main() {
    let m = Motor { hp: 150 };
    println!("{}", api::ignite(&m));
    println!("{}", inner::engine::start(&m));
    println!("{} {}", api::double(21), triple(7));
    println!("{}", describe(&inner::Eco));
    println!("{}", describe(&Mode::Sport(3)));
    println!("{}", describe(&inner::engine::Mode::Sport(9)));
    let e2 = inner::Engine { hp: m.hp + 1 };
    println!("{}", inner::ignite(&e2));
}
