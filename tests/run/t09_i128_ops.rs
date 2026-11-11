// i128 arithmetic: mul, div, rem, shifts, comparisons
fn main() {
    let a: i128 = 170141183460469231731687303715884105727; // MAX
    let b: i128 = -98765432109876543210987654321;
    let c: i128 = 12345678901234567890;
    println!("a {} b {} c {}", a, b, c);
    println!("b*c {}", b.wrapping_mul(c));
    println!("c*c {}", c * c);
    println!("b/c {} b%c {}", b / c, b % c);
    println!("a/b {} a%b {}", a / b, a % b);
    println!("-a/7 {} -a%7 {}", -a / 7, -a % 7);
    println!("a+b {} a-b wrap {}", a + b, a.wrapping_sub(b));
    for s in [0u32, 1, 63, 64, 65, 100, 127] {
        println!("shift {}: 1<<s {} b>>s {} -1<<s {} a>>s {}", s, 1i128 << s, b >> s, -1i128 << s, a >> s);
    }
    let vals: [i128; 6] = [i128::MIN, -(1 << 64), -1, 0, 1 << 64, i128::MAX];
    for &x in vals.iter() {
        for &y in vals.iter() {
            print!("{} ", if x < y { "<" } else if x == y { "=" } else { ">" });
        }
        println!();
    }
    // factorial up to 33!
    let mut f: i128 = 1;
    for i in 1..=33i128 {
        f *= i;
        println!("{}! = {}", i, f);
    }
    // gcd
    let mut x: i128 = 2 * 3 * 5 * 7 * 11 * 13 * 1000000007 * 998244353;
    let mut y: i128 = 7 * 13 * 17 * 998244353 * 1000003;
    while y != 0 {
        let t = x % y;
        x = y;
        y = t;
    }
    println!("gcd {}", x);
    println!("min / -1 wrapping {}", i128::MIN.wrapping_div(-1));
    println!("checked_mul {:?} {:?}", a.checked_mul(2), c.checked_mul(c));
    println!("neg {} abs {}", -b, b.abs());
    println!("xor {} and {} or {} not {}", b ^ c, b & c, b | c, !c);
    let parsed: i128 = "-170141183460469231731687303715884105728".parse().unwrap();
    println!("parsed {} == MIN {}", parsed, parsed == i128::MIN);
}
