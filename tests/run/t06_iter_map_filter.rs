fn main() {
    let v: Vec<i32> = (1..=12).collect();
    let sq: Vec<i32> = v.iter().map(|x| x * x).collect();
    println!("{:?}", sq);
    let ev: Vec<&i32> = v.iter().filter(|x| **x % 2 == 0).collect();
    println!("{:?}", ev);
    let both: Vec<i32> = v.iter().filter(|&&x| x % 3 == 0).map(|&x| x + 100).collect();
    println!("{:?}", both);
    let strs = ["1", "x", "22", "", "333", "-4"];
    let parsed: Vec<i32> = strs.iter().filter_map(|s| s.parse().ok()).collect();
    println!("{:?}", parsed);
    let lens: Vec<usize> = strs.iter().map(|s| s.len()).collect();
    println!("{:?}", lens);
    let mut calls = 0;
    let lazy = v.iter().map(|x| {
        calls += 1;
        x * 2
    });
    let firsts: Vec<i32> = lazy.take(3).collect();
    println!("{:?} calls {}", firsts, calls);
    let owned: Vec<String> = vec!["a", "b"].into_iter().map(String::from).collect();
    println!("{:?}", owned);
    let m: Vec<(i32, bool)> = v.iter().map(|&x| (x, x > 6)).filter(|p| p.1).collect();
    println!("{:?}", m);
    let fm: Vec<char> = "a1b2c3".chars().filter_map(|c| c.to_digit(10)).map(|d| (b'a' + d as u8) as char).collect();
    println!("{:?}", fm);
    let mw: Vec<i32> = v.iter().map_while(|&x| if x < 5 { Some(x * 10) } else { None }).collect();
    println!("{:?}", mw);
}
