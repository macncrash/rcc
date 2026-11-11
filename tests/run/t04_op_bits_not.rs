// BitAnd/BitOr/BitXor/Not/Shl/Shr overloading on a bitset type.
use std::ops::{BitAnd, BitOr, BitXor, Not, Shl, Shr};

#[derive(Clone, Copy, PartialEq, Eq)]
struct Set(u16);

impl Set {
    fn of(items: &[u8]) -> Set {
        let mut s = 0u16;
        for &i in items {
            s |= 1 << i;
        }
        Set(s)
    }
    fn show(self) -> String {
        let mut out = String::from("{");
        let mut first = true;
        for i in 0..16 {
            if self.0 & (1 << i) != 0 {
                if !first {
                    out.push(',');
                }
                out.push_str(&i.to_string());
                first = false;
            }
        }
        out.push('}');
        out
    }
}

impl BitAnd for Set {
    type Output = Set;
    fn bitand(self, o: Set) -> Set {
        Set(self.0 & o.0)
    }
}
impl BitOr for Set {
    type Output = Set;
    fn bitor(self, o: Set) -> Set {
        Set(self.0 | o.0)
    }
}
impl BitXor for Set {
    type Output = Set;
    fn bitxor(self, o: Set) -> Set {
        Set(self.0 ^ o.0)
    }
}
impl Not for Set {
    type Output = Set;
    fn not(self) -> Set {
        Set(!self.0)
    }
}
impl Shl<u32> for Set {
    type Output = Set;
    fn shl(self, n: u32) -> Set {
        Set(self.0 << n)
    }
}
impl Shr<u32> for Set {
    type Output = Set;
    fn shr(self, n: u32) -> Set {
        Set(self.0 >> n)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tri {
    T,
    F,
    U,
}
impl Not for Tri {
    type Output = Tri;
    fn not(self) -> Tri {
        match self {
            Tri::T => Tri::F,
            Tri::F => Tri::T,
            Tri::U => Tri::U,
        }
    }
}
impl BitAnd for Tri {
    type Output = Tri;
    fn bitand(self, o: Tri) -> Tri {
        match (self, o) {
            (Tri::F, _) | (_, Tri::F) => Tri::F,
            (Tri::T, Tri::T) => Tri::T,
            _ => Tri::U,
        }
    }
}

fn main() {
    let a = Set::of(&[1, 3, 5, 7]);
    let b = Set::of(&[3, 4, 5]);
    println!("{} {}", a.show(), b.show());
    println!("and {}", (a & b).show());
    println!("or {}", (a | b).show());
    println!("xor {}", (a ^ b).show());
    println!("not {}", (!a).show());
    println!("shl {}", (b << 10).show());
    println!("shr {}", (a >> 3).show());
    println!("{}", (a & !b) == Set::of(&[1, 7]));
    let vals = [Tri::T, Tri::F, Tri::U];
    for x in vals {
        for y in vals {
            print!("{:?}&{:?}={:?} ", x, y, x & y);
        }
        println!("!{:?}={:?}", x, !x);
    }
}
