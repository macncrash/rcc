fn area(r: ((i32, i32), (i32, i32))) -> i32 {
    let ((x0, y0), (x1, y1)) = r;
    (x1 - x0) * (y1 - y0)
}

fn add3(a: (i32, (i32, i32))) -> i32 {
    a.0 + (a.1).0 + a.1 .1
}

fn main() {
    let rect = ((1, 2), (5, 10));
    println!("area = {}", area(rect));
    println!("corner = {} {}", (rect.0).0, rect.1 .1);
    let nested = (1, (2, (3, (4, 5))));
    println!("{}", nested.1 .1 .1 .1);
    let (a, (b, (c, (d, e)))) = nested;
    println!("{} {} {} {} {}", a, b, c, d, e);
    println!("{}", add3((10, (20, 30))));
    let mut grid = ((0, 0), (0, 0));
    grid.0 .1 = 7;
    grid.1 .0 = 9;
    println!("{} {} {} {}", grid.0 .0, grid.0 .1, grid.1 .0, grid.1 .1);
    let pair_of_arrays = ([1, 2, 3], [4, 5]);
    println!("{} {}", pair_of_arrays.0[2], pair_of_arrays.1[1]);
    let tup_with_bool = ((true, false), 'x');
    println!("{} {} {}", (tup_with_bool.0).0, (tup_with_bool.0).1, tup_with_bool.1);
    let big = (1u8, 2u16, 3u32, 4u64, 5u128, -6i8, -7i16, -8i32, -9i64, -10i128, 1.5f32, 2.5f64);
    println!("{} {} {} {} {} {}", big.0, big.1, big.2, big.3, big.4, big.5);
    println!("{} {} {} {} {} {}", big.6, big.7, big.8, big.9, big.10, big.11);
    let copy = big;
    println!("{}", copy.4 + big.4);
}
