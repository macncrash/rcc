// io::stdin().read_to_string with input from t11_stdin_read_to_string.stdin
use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    let n = io::stdin().read_to_string(&mut input).unwrap();
    println!("read {} bytes", n);
    println!("lines {}", input.lines().count());
    let mut words = 0;
    let mut longest = "";
    for w in input.split_whitespace() {
        words += 1;
        if w.len() > longest.len() {
            longest = w;
        }
    }
    println!("words {} longest {}", words, longest);
    let nums: Vec<i64> = input.split_whitespace().filter_map(|w| w.parse().ok()).collect();
    println!("numbers {:?} sum {}", nums, nums.iter().sum::<i64>());
    for (i, line) in input.lines().enumerate() {
        println!("{:2}: [{}] rev [{}]", i, line, line.chars().rev().collect::<String>());
    }
    println!("ends with newline {}", input.ends_with('\n'));
    let mut more = String::new();
    let m = io::stdin().read_to_string(&mut more).unwrap();
    println!("second read {} bytes", m);
}
