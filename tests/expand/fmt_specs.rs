// Every std::fmt spec feature. Fills are limited to the std_mock set:
// ' ' '*' '-' '_' '0' '#' '.' '─' 'x'.
use std::fmt;

struct Pt {
    x: i32,
    y: i32,
}
impl fmt::Display for Pt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // respects width/fill via pad
        f.pad(&format!("({}, {})", self.x, self.y))
    }
}
impl fmt::Debug for Pt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Pt").field("x", &self.x).field("y", &self.y).finish()
    }
}
// ignores the formatter options entirely
struct Raw(u8);
impl fmt::Display for Raw {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "raw{}", self.0)
    }
}

fn main() {
    let n = 42i32;
    let neg = -17i64;
    let s = "hello";
    let f = 3.14159265f64;
    let w = 9usize;
    let p = 3usize;
    // positional, implicit, explicit, named, captured
    println!("{} {} {}", 1, 2, 3);
    println!("{1} {0} {1}", "a", "b");
    println!("{0} {} {} {0}", "x", "y");
    println!("{name} {age}", name = "bob", age = 30);
    println!("{n} {s} {f}");
    println!("{n:?} {s:?} {f:?}");
    println!("{:?} {:#?}", Pt { x: 1, y: 2 }, Pt { x: 3, y: 4 });
    println!("{:?}", (1, "two", [3.0, 4.5], Some('c'), None::<u8>));
    println!("{:#?}", vec![(1, "a"), (2, "b")]);
    // escapes
    println!("{{}} {{{}}} }}{{", 5);
    // width, fill, align
    println!("[{:5}] [{:<5}] [{:>5}] [{:^5}]", n, n, n, n);
    println!("[{:5}] [{:<5}] [{:>5}] [{:^5}]", s, s, s, s);
    println!("[{:*<8}] [{:->8}] [{:_^8}] [{:.^9}]", s, s, s, n);
    println!("[{:#>6}] [{:0<6}] [{:x^7}] [{:─^11}]", n, n, s, "mid");
    println!("[{:10}] [{:<10}] [{:^10}]", Pt { x: 1, y: 2 }, Pt { x: 1, y: 2 }, Pt { x: 1, y: 2 });
    println!("[{:10}] [{:>10}]", Raw(1), Raw(2));
    // sign, #, 0
    println!("{:+} {:+} {:+} {:+.2}", n, neg, 0, f);
    println!("{:05} {:05} {:+05} {:<05} {:^07}", n, neg, n, n, n);
    println!("{:#x} {:#X} {:#o} {:#b}", 255, 255, 8, 5);
    println!("{:x} {:X} {:o} {:b}", 3054, 3054, 64, 10u8);
    println!("{:08x} {:#010x} {:#06b} {:#018x}", 0xbeefu32, 0xbeefu32, 3, u64::MAX);
    println!("{:x} {:X}", -1i8, i16::MIN);
    // precision
    println!("{:.0} {:.1} {:.3} {:.10}", f, f, f, f);
    println!("{:8.3} {:<8.3} {:^8.3} {:>8.3} {:08.3}", f, f, f, f, f);
    println!("{:.3} [{:5.2}] [{:<6.1}]", s, s, s);
    println!("{:e} {:E} {:e} {:+e} {:.2e}", 1234.5f64, 0.00012f64, 100u32, 5.0f32, 123456.0f64);
    // width / precision from arguments
    println!("[{:w$}] [{:<w$}] [{:^w$.p$}]", f, s, f);
    println!("[{:1$}] [{:.*}] [{:>4$.1$}]", n, 3, f, 8, 2);
    println!("[{:0$}]", 7);
    println!("[{:width$}] [{:.prec$}] [{:>width$.prec$}]", f, f, f, width = 10, prec = 2);
    println!("[{x:>w$}] [{x:0w$}] [{x:#x}] [{x:?}]", x = 255);
    // debug flavours
    println!("{:?} {:#?}", "quo\"te\n", 'q');
    println!("{:x?} {:X?} {:#x?}", vec![10, 255], vec![10, 255], 255u8);
    println!("{:5?}|{:<8?}|{:>8?}", 1.5f64, Some(1), "ab");
    println!("{:?} {:?} {:?}", 1.0f64, 0.1f32, -0.0f64);
    println!("{:?} {:?}", f64::NAN, f64::INFINITY);
    println!("{} {} {}", u128::MAX, i128::MIN, 1e21f64);
    // Display of misc types
    println!("{} {} {} {}", true, 'z', "str", String::from("owned"));
    // format!, print!, eprint!
    let t = format!("{}-{:?}-{:>4}", 1, "a", 2);
    print!("{}", t);
    print!("\n");
    eprint!("to stderr {}\n", 1);
    eprintln!("to stderr {}", 2);
    let e = format!("");
    println!("[{}]{}", e, format!("{{}}"));
    println!();
    // nested format strings via concat! and macro forwarding
    println!(concat!("con", "cat {}"), 1);
    macro_rules! fwd {
        ($fmt:expr, $($arg:tt)*) => { println!($fmt, $($arg)*) };
    }
    fwd!("fwd {} {}", 1, 2);
    macro_rules! fwd2 {
        ($($arg:tt)*) => { format!($($arg)*) };
    }
    println!("{}", fwd2!("fwd2 {:>3} {x}", 7, x = 8));
    macro_rules! fwd3 {
        ($fmt:literal $(, $e:expr)*) => { println!($fmt $(, $e)*) };
    }
    fwd3!("fwd3 {} {}", 1 + 1, "z");
    fwd3!("fwd3 none");
    // evaluation order and single evaluation
    let mut c = 0;
    let mut next = || {
        c += 1;
        c
    };
    println!("{2} {0} {1} {0}", next(), next(), next());
    let mut d = 0;
    let mut bump = || {
        d += 10;
        d
    };
    println!("{} {}", bump(), bump());
    // implicit capture with width and precision captured
    let width = 7;
    let prec = 1;
    println!("[{f:width$.prec$}] [{n:>width$}] [{s:^width$}]");
    // unicode
    println!("{:>6}|{:<6}|{:^6}", "äö", "日本", "é");
    // long literal pieces
    println!("a\tb\\c\"d'e");
}
