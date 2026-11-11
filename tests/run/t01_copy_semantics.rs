fn modify(mut a: [i32; 3]) -> i32 {
    a[0] = 100;
    a[0] + a[1] + a[2]
}

fn modify_tuple(mut t: (u8, u8)) -> u8 {
    t.0 = 50;
    t.0 + t.1
}

fn main() {
    let a = [1, 2, 3];
    println!("{}", modify(a));
    println!("{} {} {}", a[0], a[1], a[2]);
    let mut b = a;
    b[1] = 99;
    println!("a {} {} {}", a[0], a[1], a[2]);
    println!("b {} {} {}", b[0], b[1], b[2]);
    let t = (1u8, 2u8);
    println!("{}", modify_tuple(t));
    println!("{} {}", t.0, t.1);
    let mut grid = [[0; 2]; 2];
    let mut row = grid[0];
    row[0] = 5;
    println!("row {} grid {}", row[0], grid[0][0]);
    grid[1] = row;
    println!("grid {} {}", grid[1][0], grid[1][1]);
    let mut x = 10;
    let y = x;
    x += 1;
    println!("x {} y {}", x, y);
    let mut pair = (a, t);
    pair.0[2] = -3;
    pair.1 .1 = 77;
    println!("{} {} {}", pair.0[2], a[2], pair.1 .1);
    let arrs = [a, b];
    let mut c = arrs;
    c[0][0] = 42;
    println!("{} {}", arrs[0][0], c[0][0]);
}
