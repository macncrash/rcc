fn main() {
    let x = 5;
    println!("x = {}", x);
    let mut y = x;
    y += 10;
    println!("y = {}", y);
    y -= 3;
    println!("y = {}", y);
    y *= 4;
    println!("y = {}", y);
    y /= 6;
    println!("y = {}", y);
    y %= 5;
    println!("y = {}", y);
    let z;
    if y > 1 {
        z = 100;
    } else {
        z = 200;
    }
    println!("z = {}", z);
    let (mut p, q) = (1, 2);
    p += q;
    println!("p = {} q = {}", p, q);
    let mut flags: u8 = 0;
    flags |= 1;
    flags |= 4;
    println!("flags = {}", flags);
    flags &= 5;
    flags ^= 3;
    println!("flags = {}", flags);
    flags <<= 2;
    println!("flags = {}", flags);
    flags >>= 1;
    println!("flags = {}", flags);
    let _unused = 9;
    let w: i64 = -42;
    let v: i64 = w;
    println!("v = {}", v);
}
