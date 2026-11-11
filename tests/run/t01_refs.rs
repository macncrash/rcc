fn read(r: &i32) -> i32 {
    *r + 1
}

fn longer(a: &[u8; 4], b: &[u8; 4]) -> u32 {
    let mut s = 0;
    for i in 0..4 {
        s += a[i] as u32 * b[i] as u32;
    }
    s
}

fn first<'a>(a: &'a (i32, i32)) -> &'a i32 {
    &a.0
}

fn main() {
    let x = 10;
    let r = &x;
    println!("{} {}", x, *r);
    println!("{}", read(&x));
    println!("{}", read(r));
    let rr = &r;
    println!("{}", **rr);
    let rrr = &rr;
    println!("{}", ***rrr + 5);
    let a = [1u8, 2, 3, 4];
    let b = [5u8, 6, 7, 8];
    println!("dot = {}", longer(&a, &b));
    let ar = &a;
    println!("{} {}", ar[0], ar.len());
    let t = (11, 22);
    let f = first(&t);
    println!("{}", f);
    let y = 5;
    let same = r == &10;
    println!("{}", same);
    println!("{}", *r > y);
    let refs = [&x, &y];
    println!("{} {}", refs[0], *refs[1]);
    let tr = &t;
    println!("{} {}", tr.0, tr.1);
    let (p, q) = tr;
    println!("{} {}", p, q);
    let &(m, n) = tr;
    println!("{}", m + n);
    let big = [[1i64; 3]; 3];
    let br = &big;
    println!("{}", br[1][2]);
    let elem = &big[2][0];
    println!("{}", elem);
    let c = 'q';
    let cr = &c;
    println!("{}", *cr);
}
