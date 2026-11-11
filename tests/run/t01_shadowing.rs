fn main() {
    let x = 5;
    println!("{}", x);
    let x = x * 2;
    println!("{}", x);
    {
        let x = x + 100;
        println!("inner {}", x);
        let x = 'c';
        println!("inner char {}", x);
    }
    println!("outer {}", x);
    let x = x as f64 / 4.0;
    println!("{}", x);
    let x = x > 2.0;
    println!("{}", x);
    let y = 1;
    let y = {
        let y = y + 1;
        y * 10
    };
    println!("y {}", y);
    for i in 0..3 {
        let i = i * 100;
        print!("{} ", i);
    }
    println!();
    let mut z = 1;
    for _ in 0..4 {
        let z2 = z;
        let z = z2 + 1000;
        print!("{} ", z);
    }
    z += 1;
    println!("z {}", z);
    let t = (1, 2);
    let (t, u) = t;
    println!("{} {}", t, u);
    let n: u8 = 200;
    let n: u16 = n as u16 * 2;
    let n: i64 = -(n as i64);
    println!("{}", n);
    fn f(a: i32) -> i32 {
        let a = a + 1;
        let a = a * a;
        a
    }
    println!("{}", f(4));
}
