enum Tok {
    Num(i64),
    Id(u32),
}

fn describe(n: u32) -> u32 {
    match n {
        small @ 0..=9 => {
            println!("small {}", small);
            small
        }
        mid @ (10..=99 | 1000) => {
            println!("mid {}", mid);
            mid * 2
        }
        big => {
            println!("big {}", big);
            0
        }
    }
}

fn tok(t: &Tok) -> i64 {
    match t {
        Tok::Num(n @ 1..=100) => {
            println!("num in range {}", n);
            *n
        }
        Tok::Num(n) => {
            println!("num out {}", n);
            -*n
        }
        Tok::Id(id @ (1 | 2 | 3)) => {
            println!("low id {}", id);
            *id as i64 * 100
        }
        whole @ Tok::Id(_) => {
            if let Tok::Id(k) = whole {
                println!("id {}", k);
            }
            0
        }
    }
}

fn main() {
    let mut s = 0;
    let vals = [3, 50, 1000, 500, 0];
    for k in 0..vals.len() {
        let n = vals[k];
        s += describe(n);
    }
    println!("sum {}", s);
    let toks = [Tok::Num(5), Tok::Num(500), Tok::Id(2), Tok::Id(9)];
    let mut t = 0;
    for i in 0..toks.len() {
        t += tok(&toks[i]);
    }
    println!("tok sum {}", t);
    let pair @ (a, b) = (4, 5);
    println!("{} {} {}", pair.0 + pair.1, a, b);
}
