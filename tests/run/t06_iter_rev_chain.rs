fn main() {
    let v = vec![1, 2, 3, 4, 5];
    println!("{:?}", v.iter().rev().collect::<Vec<_>>());
    let c: Vec<i32> = v.iter().copied().chain(10..13).chain(vec![100]).collect();
    println!("{:?}", c);
    let rc: Vec<i32> = v.iter().chain([6, 7].iter()).rev().cloned().collect();
    println!("{:?}", rc);
    for i in (0..5).rev() {
        print!("{} ", i);
    }
    println!();
    for i in (0..=10).rev().step_by(3) {
        print!("{} ", i);
    }
    println!();
    let s: String = "abc".chars().chain("def".chars().rev()).collect();
    println!("{}", s);
    let e: Vec<(usize, &i32)> = v.iter().enumerate().rev().collect();
    println!("{:?}", e);
    let mut it = v.iter();
    println!("{:?} {:?} {:?}", it.next(), it.next_back(), it.next());
    println!("{:?}", it.collect::<Vec<_>>());
    let once: Vec<i32> = std::iter::once(0).chain(v.iter().copied()).chain(std::iter::once(-1)).collect();
    println!("{:?}", once);
    let empty: Vec<i32> = std::iter::empty().chain(v.iter().copied().rev().take(2)).collect();
    println!("{:?}", empty);
}
