fn main() {
    let vv = vec![vec![1, 2], vec![3], vec![], vec![4, 5, 6]];
    let flat: Vec<i32> = vv.concat();
    println!("{:?}", flat);
    let j = vv.join(&0);
    println!("{:?}", j);
    let strs = vec!["alpha", "beta", "gamma"];
    println!("{}", strs.concat());
    println!("{}", strs.join(", "));
    println!("{}", strs.join(""));
    let owned: Vec<String> = strs.iter().map(|s| s.to_uppercase()).collect();
    println!("{}", owned.join("-"));
    let empty: Vec<String> = Vec::new();
    println!("[{}]", empty.join(","));
    let one = vec!["solo"];
    println!("[{}]", one.join(","));
    let nums: Vec<String> = (1..=5).map(|i| i.to_string()).collect();
    println!("{}", nums.join(" + "));
    let sl: [&[u8]; 2] = [b"ab", b"cd"];
    println!("{:?}", sl.concat());
    let lines = vec!["line1".to_string(), "line2".to_string()];
    let text = lines.join("\n");
    println!("{}", text);
    println!("len {}", text.len());
}
