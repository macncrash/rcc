fn main() {
    let vv = vec![vec![1, 2], vec![], vec![3, 4, 5]];
    let f: Vec<i32> = vv.iter().flatten().copied().collect();
    println!("{:?}", f);
    let fm: Vec<i32> = (1..4).flat_map(|x| (0..x).map(move |y| x * 10 + y)).collect();
    println!("{:?}", fm);
    let words = ["hi", "yo"];
    let chars: Vec<char> = words.iter().flat_map(|w| w.chars()).collect();
    println!("{:?}", chars);
    let opts = vec![Some(1), None, Some(3), None];
    let s: Vec<i32> = opts.iter().flatten().cloned().collect();
    println!("{:?}", s);
    let n: i32 = opts.into_iter().flatten().sum();
    println!("{}", n);
    let nested = vec![vec![vec![1], vec![2, 3]], vec![vec![4]]];
    let deep: Vec<i32> = nested.into_iter().flatten().flatten().collect();
    println!("{:?}", deep);
    let lines = "a b\nc\n\nd e f";
    let toks: Vec<&str> = lines.lines().flat_map(|l| l.split_whitespace()).collect();
    println!("{:?}", toks);
    let res: Vec<Result<i32, String>> = vec![Ok(1), Err("bad".into()), Ok(2)];
    let oks: Vec<i32> = res.iter().flatten().copied().collect();
    println!("{:?}", oks);
    let pairs: Vec<(i32, i32)> = (0..3).flat_map(|i| (i..3).map(move |j| (i, j))).collect();
    println!("{:?}", pairs);
}
