// From / Into conversions, TryFrom / TryInto, and ? with From for errors.
#![allow(dead_code)] // fields are only observed through Debug
use std::convert::TryFrom;

#[derive(Debug)]
struct Meters(f64);
struct Feet(f64);

impl From<Feet> for Meters {
    fn from(f: Feet) -> Meters {
        Meters(f.0 * 0.3048)
    }
}

#[derive(Debug, PartialEq)]
enum Token {
    Num(i64),
    Word(String),
    Flag(bool),
}

impl From<i64> for Token {
    fn from(n: i64) -> Self {
        Token::Num(n)
    }
}
impl From<&str> for Token {
    fn from(s: &str) -> Self {
        Token::Word(s.to_string())
    }
}
impl From<bool> for Token {
    fn from(b: bool) -> Self {
        Token::Flag(b)
    }
}

#[derive(Debug)]
struct Even(u32);
impl TryFrom<u32> for Even {
    type Error = String;
    fn try_from(v: u32) -> Result<Self, String> {
        if v % 2 == 0 { Ok(Even(v)) } else { Err(format!("{} is odd", v)) }
    }
}

#[derive(Debug)]
enum MyErr {
    Parse(String),
    Range(u32),
}
impl From<String> for MyErr {
    fn from(s: String) -> Self {
        MyErr::Parse(s)
    }
}

fn check(v: u32) -> Result<u32, MyErr> {
    let e = Even::try_from(v)?;
    if e.0 > 100 {
        return Err(MyErr::Range(e.0));
    }
    Ok(e.0 / 2)
}

fn tokens<T: Into<Token>>(x: T) -> Token {
    x.into()
}

fn main() {
    let m: Meters = Feet(10.0).into();
    println!("{:?}", m);
    println!("{:?}", Meters::from(Feet(1.0)));
    println!("{:?}", tokens(5i64));
    println!("{:?}", tokens("hi"));
    println!("{:?}", tokens(false));
    let t: Token = 42.into();
    println!("{}", t == Token::Num(42));
    // std conversions
    let a: i64 = 7i32.into();
    let b: f64 = 3u8.into();
    let c = u16::from(200u8);
    let d: u32 = 'A'.into();
    let s: String = "abc".into();
    println!("{} {} {} {} {}", a, b, c, d, s);
    println!("{:?} {:?}", u8::try_from(300i32).is_err(), u8::try_from(255i32));
    let r: Result<i8, _> = 128u32.try_into();
    println!("{}", r.is_err());
    let r2: Result<i8, _> = (-5i64).try_into();
    println!("{:?}", r2);
    println!("{:?}", char::from_u32(0x41));
    println!("{:?}", Even::try_from(8));
    println!("{:?}", Even::try_from(9));
    for v in [10, 7, 200, 0] {
        println!("{} -> {:?}", v, check(v));
    }
    let bx: Box<str> = Box::from("boxed");
    println!("{}", bx);
    let o: Option<i32> = 5.into();
    println!("{:?}", o);
}
