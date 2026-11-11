// A parameter (local) named like a method defined by the same expansion.
macro_rules! imp {
    ($t:ident) => {
        struct $t(f64);
        impl $t {
            fn min(&self, other: f64) -> f64 { if other < self.0 { other } else { self.0 } }
            fn max(&self, other: f64) -> f64 { if other > self.0 { other } else { self.0 } }
            fn clamp(&self, min: f64, max: f64) -> f64 {
                let mut x = self.0;
                if x < min { x = min; }
                if x > max { x = max; }
                x
            }
            fn len(&self) -> usize { let len = 3; len }
        }
    };
}
imp!(A);
macro_rules! local_fn {
    () => {{
        fn helper(x: i32) -> i32 { x + 1 }
        let helper2 = helper(1);
        helper2
    }};
}
fn main() {
    let a = A(5.0);
    println!("{} {} {} {} {}", a.min(1.0), a.max(9.0), a.clamp(6.0, 7.0), a.clamp(0.0, 3.0), a.len());
    println!("{}", local_fn!());
    println!("{} {}", 2.5f64.clamp(0.0, 1.0), (-1.0f32).min(3.0).max(0.5));
}
