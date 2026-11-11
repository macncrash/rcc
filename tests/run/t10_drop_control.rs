// Drops on early return, `?` early exit, break/continue out of loops, labelled breaks.
struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn early(flag: bool) -> u32 {
    let _a = D("early-a");
    if flag {
        let _b = D("early-b");
        println!("returning early");
        return 1;
    }
    let _c = D("early-c");
    println!("normal path");
    2
}

fn parse(s: &str) -> Result<i32, String> {
    let _g = D("parse-guard");
    let n: i32 = s.parse().map_err(|_| format!("bad '{}'", s))?;
    let _after = D("parse-after");
    Ok(n * 2)
}

fn find(o: Option<u8>) -> Option<u8> {
    let _g = D("find-guard");
    let v = o?;
    let _h = D("find-found");
    Some(v + 1)
}

fn main() {
    println!("early(true) = {}", early(true));
    println!("early(false) = {}", early(false));
    println!("{:?}", parse("21"));
    println!("{:?}", parse("x"));
    println!("{:?}", find(Some(4)));
    println!("{:?}", find(None));
    println!("-- break");
    let _outer = D("outer");
    let mut i = 0;
    loop {
        let _l = D("loop-body");
        i += 1;
        if i == 2 {
            let _b = D("before-break");
            break;
        }
        println!("iteration {}", i);
    }
    println!("-- continue");
    for k in 0..3 {
        let _c = D("for-body");
        if k == 1 {
            println!("continue at {}", k);
            continue;
        }
        println!("body {}", k);
    }
    println!("-- labelled break from nested loop");
    'out: for a in 0..3 {
        let _x = D("outer-iter");
        for b in 0..3 {
            let _y = D("inner-iter");
            if a == 1 && b == 1 {
                println!("break 'out at {} {}", a, b);
                break 'out;
            }
        }
    }
    println!("-- break with value");
    let got = loop {
        let _v = D("value-loop");
        break D("result");
    };
    println!("got {}", got.0);
    println!("end of main");
}
