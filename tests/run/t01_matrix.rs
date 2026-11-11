type M = [[i64; 3]; 3];

fn mul(a: &M, b: &M) -> M {
    let mut c = [[0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let mut s = 0;
            for k in 0..3 {
                s += a[i][k] * b[k][j];
            }
            c[i][j] = s;
        }
    }
    c
}

fn det(m: &M) -> i64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn show(m: &M) {
    for row in m {
        println!("{} {} {}", row[0], row[1], row[2]);
    }
}

fn main() {
    let a: M = [[1, 2, 3], [4, 5, 6], [7, 8, 10]];
    let id: M = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
    show(&mul(&a, &id));
    let a2 = mul(&a, &a);
    show(&a2);
    println!("det a = {}", det(&a));
    println!("det a2 = {}", det(&a2));
    let fibm: M = [[1, 1, 0], [1, 0, 0], [0, 0, 1]];
    let mut p = id;
    for i in 1..=60 {
        p = mul(&p, &fibm);
        if i % 15 == 0 {
            println!("fib({}) = {}", i, p[0][1]);
        }
    }
    let mut acc = id;
    let mut base = a;
    let mut e = 5;
    while e > 0 {
        if e & 1 == 1 {
            acc = mul(&acc, &base);
        }
        base = mul(&base, &base);
        e >>= 1;
    }
    show(&acc);
}
