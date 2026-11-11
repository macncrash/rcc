// io::Error kinds, custom io errors, ? propagation through functions returning io::Result
use std::fs;
use std::io::{self, ErrorKind, Read};

fn read_number(path: &str) -> io::Result<i64> {
    let mut s = String::new();
    fs::File::open(path)?.read_to_string(&mut s)?;
    s.trim().parse::<i64>().map_err(|e| io::Error::new(ErrorKind::InvalidData, format!("bad number {:?}: {}", s.trim(), e)))
}

fn sum_files(paths: &[&str]) -> io::Result<i64> {
    let mut total = 0;
    for p in paths {
        let v = read_number(p)?;
        println!("  {} -> {}", p, v);
        total += v;
    }
    Ok(total)
}

fn main() {
    let dir = "t11_io_errors_tmp";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir(dir).unwrap();
    let a = format!("{}/a", dir);
    let b = format!("{}/b", dir);
    let c = format!("{}/c", dir);
    fs::write(&a, "40\n").unwrap();
    fs::write(&b, " 2 ").unwrap();
    fs::write(&c, "two").unwrap();
    println!("sum a+b {:?}", sum_files(&[&a, &b]).ok());
    match sum_files(&[&a, &c, &b]) {
        Ok(v) => println!("unexpected {}", v),
        Err(e) => println!("error kind {:?} msg {}", e.kind(), e),
    }
    match sum_files(&[&a, "t11_io_errors_tmp/missing"]) {
        Ok(v) => println!("unexpected {}", v),
        Err(e) => println!("missing kind {:?} is NotFound {}", e.kind(), e.kind() == ErrorKind::NotFound),
    }
    let custom = io::Error::new(ErrorKind::Other, "custom failure");
    println!("custom {:?} {}", custom.kind(), custom);
    let from_kind: io::Error = ErrorKind::UnexpectedEof.into();
    println!("from kind {:?}", from_kind.kind());
    println!("create_dir existing {:?}", fs::create_dir(dir).unwrap_err().kind());
    let mut short = io::Cursor::new(vec![1u8, 2]);
    let mut buf = [0u8; 4];
    println!("read_exact short {:?}", short.read_exact(&mut buf).unwrap_err().kind());
    fs::remove_dir_all(dir).unwrap();
    println!("done");
}
