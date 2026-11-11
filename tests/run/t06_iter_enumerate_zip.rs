fn main() {
    let names = ["ann", "ben", "cal"];
    let ages = [31, 42, 27, 99];
    for (i, n) in names.iter().enumerate() {
        println!("{} {}", i, n);
    }
    for (n, a) in names.iter().zip(ages.iter()) {
        println!("{} is {}", n, a);
    }
    let z: Vec<(usize, (&&str, &i32))> = names.iter().zip(ages.iter()).enumerate().collect();
    println!("{:?}", z);
    let dot: i32 = [1, 2, 3].iter().zip([4, 5, 6].iter()).map(|(a, b)| a * b).sum();
    println!("dot {}", dot);
    let (small, idx): (Vec<i32>, Vec<usize>) = ages.iter().enumerate().filter(|(_, &a)| a < 40).map(|(i, &a)| (a, i)).unzip();
    println!("{:?} {:?}", small, idx);
    let pairs = vec![(1, 'a'), (2, 'b'), (3, 'c')];
    let (ns, cs): (Vec<i32>, Vec<char>) = pairs.into_iter().unzip();
    println!("{:?} {:?}", ns, cs);
    let z3: Vec<(i32, char, bool)> = ns.iter().zip(cs.iter()).zip([true, false, true]).map(|((a, b), c)| (*a, *b, c)).collect();
    println!("{:?}", z3);
    let shifted: Vec<i32> = ns.iter().zip(ns.iter().skip(1)).map(|(a, b)| b - a).collect();
    println!("{:?}", shifted);
    for (i, c) in "xyz".chars().enumerate().skip(1) {
        println!("{} {}", i, c);
    }
}
