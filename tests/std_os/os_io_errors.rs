// io::Error Display/Debug/kind for OS, custom and simple errors; Box<dyn Error> conversions and `?`
use std::error::Error;
use std::fs;
use std::io::{self, ErrorKind, Read, Write};

fn parse_num(s: &str) -> Result<i64, Box<dyn Error>> {
    let n: i64 = s.trim().parse()?;
    if n < 0 {
        return Err("negative".into());
    }
    if n > 1000 {
        return Err(format!("too big: {}", n).into());
    }
    Ok(n)
}

fn open_missing() -> Result<String, Box<dyn Error + Send + Sync>> {
    let s = fs::read_to_string("os_io_errors_missing.txt")?;
    Ok(s)
}

fn io_chain() -> io::Result<usize> {
    let mut f = fs::File::open("os_io_errors_missing_dir/x")?;
    let mut s = String::new();
    f.read_to_string(&mut s)
}

fn main() {
    let e = fs::File::open("os_io_errors_missing.txt").unwrap_err();
    println!("display: {}", e);
    println!("debug: {:?}", e);
    println!("kind: {:?} / {} raw {:?}", e.kind(), e.kind(), e.raw_os_error());
    println!("to_string: {}", e.to_string());
    let c = io::Error::new(ErrorKind::InvalidData, "custom failure");
    println!("custom: {} | {:?} | kind {:?} raw {:?}", c, c, c.kind(), c.raw_os_error());
    println!("custom get_ref: {:?}", c.get_ref().map(|e| e.to_string()));
    let inner = c.into_inner().unwrap();
    println!("into_inner: {}", inner);
    let o = io::Error::other(format!("other {}", 5));
    println!("other: {} | {:?}", o, o);
    let k: io::Error = ErrorKind::BrokenPipe.into();
    println!("simple: {} | {:?}", k, k);
    for code in [1, 2, 13, 17, 20, 21, 22, 28, 66] {
        let e = io::Error::from_raw_os_error(code);
        println!("os {}: kind {:?}", code, e.kind());
    }
    let nested = io::Error::new(ErrorKind::NotFound, io::Error::from_raw_os_error(2));
    println!("nested: {}", nested);
    let mut buf = [0u8; 8];
    let r = (&b"abc"[..]).read_exact(&mut buf).unwrap_err();
    println!("read_exact: {} | {:?}", r, r);
    let mut small = [0u8; 3];
    let mut w: &mut [u8] = &mut small;
    let r = w.write_all(b"abcdef").unwrap_err();
    println!("write_all: {} | {:?} small {:?}", r, r, small);
    let mut s = String::from("keep");
    let r = (&[b'o', b'k', 0xff][..]).read_to_string(&mut s).unwrap_err();
    println!("bad utf8: {} | kind {:?} | buffer kept {:?}", r, r.kind(), s);
    for k in [ErrorKind::NotFound, ErrorKind::PermissionDenied, ErrorKind::AlreadyExists, ErrorKind::WouldBlock, ErrorKind::InvalidInput, ErrorKind::InvalidData, ErrorKind::TimedOut, ErrorKind::WriteZero, ErrorKind::Interrupted, ErrorKind::Unsupported, ErrorKind::UnexpectedEof, ErrorKind::OutOfMemory, ErrorKind::Other, ErrorKind::IsADirectory, ErrorKind::NotADirectory, ErrorKind::DirectoryNotEmpty] {
        println!("{:?}: {}", k, k);
    }
    for s in ["12", " 7 ", "-3", "5000", "x1"] {
        match parse_num(s) {
            Ok(n) => println!("parse {:?} -> {}", s, n),
            Err(e) => println!("parse {:?} -> error {} / {:?}", s, e, e.to_string()),
        }
    }
    match open_missing() {
        Ok(_) => println!("unexpected"),
        Err(e) => println!("boxed io: {} | {:?}", e, e),
    }
    match io_chain() {
        Ok(_) => println!("unexpected"),
        Err(e) => println!("chain: {:?}", e.kind()),
    }
    let be: Box<dyn Error> = Box::new(io::Error::from_raw_os_error(13));
    println!("box dyn: {} source none {}", be, be.source().is_none());
    let r: Result<(), Box<dyn Error>> = Err("plain".into());
    println!("{:?}", r);
    let fe: Box<dyn Error> = From::from(std::fmt::Error);
    println!("fmt error: {}", fe);
}
