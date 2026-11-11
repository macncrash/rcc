// io::stdin().lock().lines() and read_line, parsing a small command language
use std::io::{self, BufRead};

fn main() {
    let stdin = io::stdin();
    let mut first = String::new();
    let n = stdin.lock().read_line(&mut first).unwrap();
    println!("header ({} bytes): {:?}", n, first.trim_end());
    let mut stack: Vec<i64> = Vec::new();
    for (lineno, line) in stdin.lock().lines().enumerate() {
        let line = line.unwrap();
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            println!("{}: skip", lineno);
            continue;
        }
        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap();
        match cmd {
            "push" => {
                for p in parts {
                    stack.push(p.parse().unwrap());
                }
            }
            "add" => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                stack.push(a + b);
            }
            "mul" => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                stack.push(a * b);
            }
            "dup" => {
                let a = *stack.last().unwrap();
                stack.push(a);
            }
            other => println!("{}: unknown command {}", lineno, other),
        }
        println!("{}: {} -> {:?}", lineno, cmd, stack);
    }
    let mut rest = String::new();
    let eof = stdin.lock().read_line(&mut rest).unwrap();
    println!("read_line at eof returns {}", eof);
}
