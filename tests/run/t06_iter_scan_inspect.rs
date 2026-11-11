fn main() {
    let v = [1, 2, 3, 4, 5, 6];
    let prefix: Vec<i32> = v.iter().scan(0, |acc, &x| {
        *acc += x;
        Some(*acc)
    }).collect();
    println!("{:?}", prefix);
    let until: Vec<i32> = v.iter().scan(1, |acc, &x| {
        *acc *= x;
        if *acc > 100 { None } else { Some(*acc) }
    }).collect();
    println!("{:?}", until);
    let mut seen = Vec::new();
    let total: i32 = v
        .iter()
        .inspect(|x| seen.push(**x))
        .filter(|x| **x % 2 == 1)
        .sum();
    println!("total {} seen {:?}", total, seen);
    let mut log = String::new();
    let out: Vec<i32> = v
        .iter()
        .map(|x| x * 3)
        .inspect(|x| log.push_str(&format!("[{}]", x)))
        .take_while(|x| *x < 12)
        .collect();
    println!("{:?} {}", out, log);
    let fib: Vec<u64> = (0..10).scan((0u64, 1u64), |st, _| {
        let r = st.0;
        *st = (st.1, st.0 + st.1);
        Some(r)
    }).collect();
    println!("{:?}", fib);
}
