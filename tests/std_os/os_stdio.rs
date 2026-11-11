// stdin (read_line/lines/read/read_to_end through one shared buffer), stdout ordering, is_terminal
use std::io::{self, BufRead, IsTerminal, Read, Write};

fn main() {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.read_line(&mut line).unwrap();
    println!("read_line {:?}", line);
    let mut lock = stdin.lock();
    let mut two = String::new();
    lock.read_line(&mut two).unwrap();
    lock.read_line(&mut two).unwrap();
    println!("two lines {:?}", two);
    let mut buf = [0u8; 5];
    lock.read_exact(&mut buf).unwrap();
    println!("read_exact {:?}", String::from_utf8_lossy(&buf));
    drop(lock);
    let mut l3 = String::new();
    io::stdin().lock().read_line(&mut l3).unwrap();
    println!("rest of line {:?}", l3);
    let some: Vec<String> = io::stdin().lock().lines().take(3).map(|l| l.unwrap()).collect();
    println!("lines {:?}", some);
    let mut rest = Vec::new();
    io::stdin().read_to_end(&mut rest).unwrap();
    println!("read_to_end {} bytes, newlines {}, last {:?}", rest.len(), rest.iter().filter(|&&b| b == b'\n').count(), rest.last().map(|&b| b as char));
    let mut after = String::new();
    println!("eof read_line {} read_to_string {}", io::stdin().read_line(&mut after).unwrap(), io::stdin().read_to_string(&mut after).unwrap());
    println!("terminal stdin {} stdout {}", io::stdin().is_terminal(), io::stdout().is_terminal());

    let out = io::stdout();
    let mut h = out.lock();
    write!(h, "a").unwrap();
    print!("b");
    h.write_all(b"c\n").unwrap();
    drop(h);
    eprintln!("stderr line (not compared)");
    let mut err = io::stderr().lock();
    writeln!(err, "locked stderr").unwrap();
    for i in 0..3 {
        print!("{}", i);
        io::stdout().write_all(b"-").unwrap();
    }
    println!();
    let big = "z".repeat(100_000);
    io::stdout().write_all(big.as_bytes()).unwrap();
    println!();
    println!("{:?}", io::stdout());
    print!("unterminated at exit");
}
