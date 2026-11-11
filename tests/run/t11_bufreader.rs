// BufReader over a File: read_line, lines, read_until, split, Cursor
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Cursor, Read};

fn main() {
    let dir = "t11_bufreader_tmp";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir(dir).unwrap();
    let path = format!("{}/in.txt", dir);
    let mut content = String::new();
    for i in 0..20 {
        content.push_str(&format!("{};{};{}\n", i, i * i, if i % 2 == 0 { "even" } else { "odd" }));
    }
    content.push_str("no-newline-at-end");
    fs::write(&path, &content).unwrap();

    let mut r = BufReader::new(File::open(&path).unwrap());
    let mut line = String::new();
    let n = r.read_line(&mut line).unwrap();
    println!("first read_line {} {:?}", n, line);
    line.clear();
    r.read_line(&mut line).unwrap();
    println!("second {:?}", line);
    let mut total = 0;
    let mut evens = 0;
    for l in r.lines() {
        let l = l.unwrap();
        let fields: Vec<&str> = l.split(';').collect();
        if fields.len() == 3 {
            total += fields[1].parse::<i32>().unwrap();
            if fields[2] == "even" {
                evens += 1;
            }
        } else {
            println!("odd line {:?}", l);
        }
    }
    println!("sum of squares {} evens {}", total, evens);

    let mut r2 = BufReader::with_capacity(8, File::open(&path).unwrap());
    let mut buf = Vec::new();
    let mut count = 0;
    loop {
        buf.clear();
        let n = r2.read_until(b';', &mut buf).unwrap();
        if n == 0 {
            break;
        }
        count += 1;
        if count <= 4 {
            println!("read_until {:?}", String::from_utf8_lossy(&buf));
        }
    }
    println!("segments {}", count);

    let cur = Cursor::new(b"alpha,beta,,gamma".to_vec());
    for part in cur.split(b',') {
        println!("split {:?}", String::from_utf8(part.unwrap()).unwrap());
    }
    let mut c2 = Cursor::new("one\ntwo\r\nthree");
    let mut all = String::new();
    c2.read_to_string(&mut all).unwrap();
    println!("cursor read {:?} pos {}", all, c2.position());
    let lines: Vec<String> = BufReader::new(Cursor::new("x\ny\r\nz")).lines().map(|l| l.unwrap()).collect();
    println!("lines strip crlf {:?}", lines);
    fs::remove_dir_all(dir).unwrap();
}
