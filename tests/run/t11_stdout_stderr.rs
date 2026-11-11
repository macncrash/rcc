// stdout/stderr writes, locking, flushing, print! without newline (only stdout is checked)
use std::io::{self, Write};

fn main() {
    print!("no newline yet...");
    io::stdout().flush().unwrap();
    println!(" done");
    eprintln!("this goes to stderr and is not compared");
    eprint!("stderr without newline\n");
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    for i in 0..5 {
        write!(lock, "{} ", i * i).unwrap();
    }
    writeln!(lock).unwrap();
    lock.write_all(b"raw bytes\n").unwrap();
    lock.flush().unwrap();
    drop(lock);
    let mut err = io::stderr();
    writeln!(err, "error stream {}", 42).unwrap();
    err.flush().unwrap();
    let mut out = io::stdout();
    let n = out.write(b"partial write\n").unwrap();
    println!("wrote {}", n);
    // interleaving println and direct writes keeps order on stdout
    for i in 0..3 {
        println!("println {}", i);
        io::stdout().write_all(format!("write_all {}\n", i).as_bytes()).unwrap();
    }
    print!("a");
    print!("b");
    print!("c\n");
    let big: String = (0..2000).map(|i| char::from(b'a' + (i % 26) as u8)).collect();
    println!("{}", big);
    println!("len {}", big.len());
}
