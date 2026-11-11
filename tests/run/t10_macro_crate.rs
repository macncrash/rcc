// $crate paths, #[macro_use] modules, textual scoping, macro shadowing, #[macro_export].
pub fn helper_value() -> i32 {
    40
}

pub mod lib {
    pub fn add(a: i32, b: i32) -> i32 {
        a + b
    }
    #[macro_export]
    macro_rules! exported_add {
        ($a:expr, $b:expr) => {
            $crate::lib::add($a, $b) + $crate::helper_value()
        };
    }
}

#[macro_use]
mod macros {
    macro_rules! greet {
        ($n:expr) => {
            format!("hi {}", $n)
        };
    }
    macro_rules! twice {
        ($e:expr) => {
            ($e, $e)
        };
    }
}

// textual scoping: a macro is only visible after its definition
macro_rules! which {
    () => {
        "first"
    };
}

fn use_first() -> &'static str {
    which!()
}

macro_rules! which {
    () => {
        "second (shadowed)"
    };
}

fn use_second() -> &'static str {
    which!()
}

mod scoped {
    pub fn inner() -> String {
        macro_rules! local_only {
            ($x:expr) => {
                $x * 1000
            };
        }
        format!("{} {}", local_only!(3), greet!("scoped"))
    }
}

fn main() {
    println!("{}", exported_add!(1, 2));
    println!("{}", crate::exported_add!(10, 20));
    println!("{}", greet!("bob"));
    println!("{:?}", twice!(7 + 1));
    println!("{}", use_first());
    println!("{}", use_second());
    println!("{}", scoped::inner());
    {
        macro_rules! which {
            () => {
                "block-local"
            };
        }
        println!("{}", which!());
    }
    println!("{}", which!());
}
