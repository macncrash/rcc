fn main() {
    for i in 1..=9 {
        for j in 1..=i {
            print!("{}*{}={} ", j, i, i * j);
        }
        println!();
    }
    let mut count = 0;
    for a in 1..30 {
        for b in a..30 {
            for c in b..30 {
                if a * a + b * b == c * c {
                    println!("triple {} {} {}", a, b, c);
                    count += 1;
                }
            }
        }
    }
    println!("triples = {}", count);
    let mut i = 0;
    while i < 3 {
        for j in 0..3 {
            let mut k = 0;
            loop {
                if k > j {
                    break;
                }
                print!("({},{},{})", i, j, k);
                k += 1;
            }
        }
        println!();
        i += 1;
    }
    for row in 0..5 {
        for _ in 0..(4 - row) {
            print!(" ");
        }
        for _ in 0..(2 * row + 1) {
            print!("*");
        }
        println!();
    }
}
