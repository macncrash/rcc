// Macros that generate items: structs, enums, impls, trait impls, functions, tests of ops.
use std::ops::Add;

macro_rules! newtype {
    ($name:ident($inner:ty)) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        struct $name($inner);
        impl Add for $name {
            type Output = $name;
            fn add(self, o: $name) -> $name {
                $name(self.0 + o.0)
            }
        }
        impl $name {
            fn get(self) -> $inner {
                self.0
            }
        }
    };
}
newtype!(Meters(f64));
newtype!(Count(u32));

macro_rules! getters {
    ($s:ident { $($f:ident : $t:ty),* }) => {
        struct $s { $($f: $t),* }
        impl $s {
            $(fn $f(&self) -> $t { self.$f })*
        }
    };
}
getters!(Config { width: u32, height: u32, depth: u8 });

macro_rules! opcodes {
    ($($name:ident = $val:expr),* $(,)?) => {
        #[derive(Debug, Clone, Copy)]
        enum Opcode { $($name = $val),* }
        impl Opcode {
            fn from_u8(v: u8) -> Option<Opcode> {
                $(if v == $val { return Some(Opcode::$name); })*
                None
            }
            fn mnemonic(self) -> &'static str {
                match self { $(Opcode::$name => stringify!($name)),* }
            }
        }
    };
}
opcodes! { Nop = 0x90, Ret = 0xc3, Int3 = 0xcc, Hlt = 0xf4 }

macro_rules! def_fns {
    ($($name:ident => $body:expr;)*) => {
        $(fn $name(x: i64) -> i64 { let f = $body; f(x) })*
    };
}
def_fns! {
    inc => |x| x + 1;
    dbl => |x| x * 2;
    sq  => |x| x * x;
}

trait Named {
    fn type_name() -> &'static str;
}
macro_rules! impl_named {
    ($($t:ty),*) => { $(impl Named for $t { fn type_name() -> &'static str { stringify!($t) } })* };
}
impl_named!(u8, i64, bool, String);

fn main() {
    let m = Meters(1.5) + Meters(2.25);
    println!("{:?} {}", m, m.get());
    let c = Count(3) + Count(4) + Count(5);
    println!("{:?} {} {}", c, c.get(), c == Count(12));
    let cfg = Config { width: 640, height: 480, depth: 24 };
    println!("{}x{}x{}", cfg.width(), cfg.height(), cfg.depth());
    for b in [0x90u8, 0xc3, 0x00, 0xcc, 0xf4] {
        match Opcode::from_u8(b) {
            Some(op) => println!("{:#x} -> {:?} {} {}", b, op, op.mnemonic(), op as u8),
            None => println!("{:#x} -> unknown", b),
        }
    }
    println!("{} {} {}", inc(5), dbl(5), sq(5));
    println!("{} {} {} {}", u8::type_name(), i64::type_name(), bool::type_name(), String::type_name());
}
