enum Stack {
    Nil,
    Top(u32, u32),
}

fn pop(s: &mut Stack) -> Stack {
    match *s {
        Stack::Nil => Stack::Nil,
        Stack::Top(v, depth) => {
            if depth == 0 {
                *s = Stack::Nil;
            } else {
                *s = Stack::Top(v * 2, depth - 1);
            }
            Stack::Top(v, depth)
        }
    }
}

fn main() {
    let mut st = Stack::Top(1, 6);
    while let Stack::Top(v, d) = pop(&mut st) {
        println!("popped {} depth {}", v, d);
    }
    let mut n: u64 = 27;
    let mut steps = 0;
    while let 2..=u64::MAX = n {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        steps += 1;
        if steps % 10 == 0 {
            println!("step {} n {}", steps, n);
        }
    }
    println!("collatz steps {}", steps);
    let mut pair = (0, 10);
    while let (a, b @ 0..=100) = pair {
        if a >= b {
            break;
        }
        println!("{} {}", a, b);
        pair = (a + 2, b - 1);
    }
    let mut cnt = 0;
    while let true = cnt < 3 {
        cnt += 1;
    }
    println!("cnt {}", cnt);
}
