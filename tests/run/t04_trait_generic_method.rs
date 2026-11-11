// Generic methods inside traits and impls; generic traits.
trait Visitor {
    fn visit_num(&mut self, n: i64);
    fn visit_all<I: Copy + Into<i64>>(&mut self, xs: &[I]) {
        for x in xs {
            self.visit_num((*x).into());
        }
    }
}

struct Sum(i64);
struct Max(i64);

impl Visitor for Sum {
    fn visit_num(&mut self, n: i64) {
        self.0 += n;
    }
}
impl Visitor for Max {
    fn visit_num(&mut self, n: i64) {
        if n > self.0 {
            self.0 = n;
        }
    }
}

trait Convert<T> {
    fn convert(&self) -> T;
}

struct Celsius(f64);

impl Convert<f64> for Celsius {
    fn convert(&self) -> f64 {
        self.0 * 9.0 / 5.0 + 32.0
    }
}
impl Convert<i32> for Celsius {
    fn convert(&self) -> i32 {
        self.0 as i32
    }
}
impl Convert<bool> for Celsius {
    fn convert(&self) -> bool {
        self.0 > 0.0
    }
}

struct Holder {
    v: i32,
}
impl Holder {
    fn map<R, F: Fn(i32) -> R>(&self, f: F) -> R {
        f(self.v)
    }
    fn pair_with<T: Copy>(&self, t: T) -> (i32, T) {
        (self.v, t)
    }
}

fn main() {
    let mut s = Sum(0);
    s.visit_all(&[1u8, 2, 3]);
    s.visit_all(&[-10i32, 100]);
    s.visit_num(1000);
    println!("{}", s.0);
    let mut m = Max(i64::MIN);
    m.visit_all(&[5u32, 9, 2]);
    m.visit_all(&[-4i16]);
    println!("{}", m.0);

    let c = Celsius(37.5);
    let f: f64 = c.convert();
    let i: i32 = c.convert();
    let b: bool = c.convert();
    println!("{} {} {}", f, i, b);
    println!("{}", <Celsius as Convert<f64>>::convert(&Celsius(-40.0)));
    println!("{}", Convert::<bool>::convert(&Celsius(-1.0)));

    let h = Holder { v: 7 };
    println!("{}", h.map(|x| x * 3));
    println!("{}", h.map(|x| x > 5));
    let (a, b) = h.pair_with('q');
    println!("{} {}", a, b);
    let (a2, b2) = h.pair_with(2.5);
    println!("{} {}", a2, b2);
}
