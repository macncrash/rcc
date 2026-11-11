// Compound assignment operator traits.
use std::ops::{AddAssign, BitXorAssign, DivAssign, MulAssign, ShlAssign, SubAssign};

#[derive(Debug, Clone, Copy)]
struct Money {
    cents: i64,
}

impl AddAssign for Money {
    fn add_assign(&mut self, o: Money) {
        self.cents += o.cents;
    }
}
impl SubAssign for Money {
    fn sub_assign(&mut self, o: Money) {
        self.cents -= o.cents;
    }
}
impl MulAssign<i64> for Money {
    fn mul_assign(&mut self, k: i64) {
        self.cents *= k;
    }
}
impl DivAssign<i64> for Money {
    fn div_assign(&mut self, k: i64) {
        self.cents /= k;
    }
}
impl AddAssign<i64> for Money {
    fn add_assign(&mut self, c: i64) {
        self.cents += c;
    }
}

#[derive(Debug)]
struct Bits(u32);
impl BitXorAssign<u32> for Bits {
    fn bitxor_assign(&mut self, m: u32) {
        self.0 ^= m;
    }
}
impl ShlAssign<u32> for Bits {
    fn shl_assign(&mut self, n: u32) {
        self.0 <<= n;
    }
}

fn add_n<T: AddAssign + Copy>(acc: &mut T, v: T, n: u32) {
    for _ in 0..n {
        *acc += v;
    }
}

struct Wallet {
    m: Money,
}

fn main() {
    let mut m = Money { cents: 100 };
    m += Money { cents: 250 };
    println!("{:?}", m);
    m -= Money { cents: 50 };
    println!("{:?}", m);
    m *= 3;
    println!("{:?}", m);
    m /= 4;
    println!("{:?}", m);
    m += 1;
    println!("{:?}", m);
    let mut w = Wallet { m: Money { cents: 0 } };
    w.m += Money { cents: 7 };
    w.m *= 6;
    println!("{}", w.m.cents);
    let mut arr = [Money { cents: 1 }, Money { cents: 2 }];
    arr[1] += Money { cents: 40 };
    arr[0] *= -5;
    println!("{:?} {:?}", arr[0], arr[1]);

    let mut b = Bits(0b1010);
    b ^= 0b0110;
    println!("{:?}", b);
    b <<= 4;
    println!("{:?}", b);

    let mut x = 5i32;
    add_n(&mut x, 3, 4);
    println!("{}", x);
    let mut mm = Money { cents: 1 };
    add_n(&mut mm, Money { cents: 10 }, 3);
    println!("{:?}", mm);
    let mut f = 0.5f64;
    add_n(&mut f, 0.25, 2);
    println!("{}", f);
}
