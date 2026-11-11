// rcc-flags: --env RCC_TEST_VAR=hello_env
// vec!, matches!, thread_local!, env!, option_env!, concat!, stringify!,
// line!/column!/file!/module_path!, include_str!, assert family, dbg!.
#![allow(dead_code)]
use std::cell::{Cell, RefCell};

thread_local! {
    static COUNTER: Cell<u32> = const { Cell::new(5) };
    /// doc comment on a thread local
    pub static NAMES: RefCell<Vec<String>> = RefCell::new(Vec::new());
}
thread_local!(static ONE: u8 = 1);

mod inner {
    pub fn path() -> &'static str {
        module_path!()
    }
}

fn side(log: &mut Vec<&'static str>, s: &'static str) -> bool {
    log.push(s);
    true
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(i64),
    Op(char),
    Eof,
}

fn main() {
    // vec!
    let a: Vec<i32> = vec![];
    let b = vec![1, 2, 3];
    let c = vec![0u8; 4];
    let d = vec![String::from("x"); 2];
    let e = vec![vec![1, 2], vec![], vec![3]];
    let v = 7;
    let f = vec![v, v + 1, { let v = 2; v }];
    println!("{:?} {:?} {:?} {:?} {:?} {:?}", a, b, c, d, e, f);
    let boxed: Vec<Box<dyn Fn(i32) -> i32>> = vec![Box::new(|x| x + 1), Box::new(|x| x * 2)];
    println!("{:?}", boxed.iter().map(|g| g(10)).collect::<Vec<_>>());
    // matches!
    let t = Tok::Num(3);
    println!("{} {} {}", matches!(t, Tok::Num(_)), matches!(t, Tok::Op('+') | Tok::Op('-')), matches!(t, Tok::Num(n) if n > 2));
    println!("{}", matches!(Tok::Eof, Tok::Eof,));
    let ch = 'x';
    println!("{}", matches!(ch, 'a'..='z' | 'A'..='Z' | '_'));
    // thread_local!
    COUNTER.with(|c| c.set(c.get() + 1));
    println!("{}", COUNTER.with(|c| c.get()));
    COUNTER.set(10);
    println!("{} {}", COUNTER.get(), ONE.with(|o| *o));
    NAMES.with(|n| n.borrow_mut().push("a".into()));
    NAMES.with_borrow_mut(|n| n.push("b".into()));
    println!("{:?}", NAMES.with_borrow(|n| n.clone()));
    // env
    println!("{}", env!("RCC_TEST_VAR"));
    println!("{:?} {:?}", option_env!("RCC_TEST_VAR"), option_env!("RCC_SURELY_UNSET_VAR"));
    // concat!, stringify!
    println!("{}", concat!("a", 1, 2.5, true, 'c', -3, "z"));
    println!("{}", concat!());
    println!("{}", stringify!(a + b * c));
    println!("{}", stringify!(Vec<u8>));
    println!("{}", stringify!(x));
    // line!, column!, file!, module_path!
    let l = line!();
    let col = column!();
    println!("{} {}", l, col);
    println!("{}", file!().ends_with("builtins.rs"));
    println!("{} {}", module_path!(), inner::path());
    macro_rules! where_am_i {
        () => { (line!(), column!()) };
    }
    println!("{:?}", where_am_i!());
    // asserts that pass; debug_assert must not evaluate its argument
    let mut log = vec![];
    assert!(side(&mut log, "assert"));
    assert!(1 + 1 == 2, "math is broken: {}", 1 + 1);
    assert_eq!(b.len(), 3);
    assert_eq!(b[0], 1, "first elem {}", b[0]);
    assert_ne!(b[1], b[2]);
    assert_ne!(b[1], 5, "ne msg");
    debug_assert!(side(&mut log, "debug_assert"));
    debug_assert_eq!({ log.push("dae"); 1 }, 1);
    debug_assert_ne!(1, 2);
    println!("{:?}", log);
    // dbg! returns its argument (output goes to stderr)
    let x = dbg!(2 * 21);
    let (p, q) = dbg!(1, "two");
    println!("{} {} {}", x, p, q);
    // std:: paths to builtins
    let s = std::format!("{}", 5);
    std::println!("{}", s);
    let w = ::std::vec![1u8];
    core::assert!(w.len() == 1);
    println!("{}", std::concat!("x", "y"));
    if false {
        unreachable!();
    }
    if false {
        unimplemented!("never {}", 1);
    }
    if false {
        todo!();
    }
    let r: Result<i32, String> = Ok(1);
    let val = match r {
        Ok(v) => v,
        Err(e) => panic!("bad {}", e),
    };
    println!("{}", val);
}
