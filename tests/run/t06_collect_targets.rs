use std::collections::{HashMap, HashSet};
fn main() {
    let v: Vec<u8> = (b'a'..=b'e').collect();
    println!("{:?}", v);
    let s: String = v.iter().map(|&b| b as char).collect();
    println!("{}", s);
    let s2: String = vec!["ab", "cd", "ef"].into_iter().collect();
    println!("{}", s2);
    let s3: String = vec![String::from("x"), String::from("y")].into_iter().collect();
    println!("{}", s3);
    let m: HashMap<&str, usize> = ["one", "three", "five"].iter().map(|w| (*w, w.len())).collect();
    let mut keys: Vec<_> = m.iter().collect();
    keys.sort();
    println!("{:?}", keys);
    println!("{:?}", m.get("three"));
    let hs: HashSet<i32> = [3, 1, 3, 2, 1, 5].iter().copied().collect();
    let mut hv: Vec<i32> = hs.into_iter().collect();
    hv.sort();
    println!("{:?}", hv);
    let chars: HashSet<char> = "mississippi".chars().collect();
    let mut cv: Vec<char> = chars.iter().copied().collect();
    cv.sort();
    println!("{:?} {}", cv, chars.contains(&'s'));
    let mut counts: HashMap<char, u32> = HashMap::new();
    for c in "mississippi".chars() {
        *counts.entry(c).or_insert(0) += 1;
    }
    let mut cs: Vec<(char, u32)> = counts.into_iter().collect();
    cs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!("{:?}", cs);
    let r: Result<Vec<i32>, String> = vec![Ok(1), Err("e1".to_string()), Err("e2".to_string())].into_iter().collect();
    println!("{:?}", r);
    let o: Option<Vec<i32>> = vec![Some(1), Some(2)].into_iter().collect();
    println!("{:?}", o);
    let o2: Option<Vec<i32>> = vec![Some(1), None].into_iter().collect();
    println!("{:?}", o2);
    let bx: Box<[i32]> = (1..4).collect();
    println!("{:?} {}", bx, bx.len());
    let mut ext = vec![0];
    ext.extend((1..3).map(|x| x * 7));
    println!("{:?}", ext);
    let from_arr = Vec::from([9, 8]);
    let from_slice: Vec<i32> = (&[1, 2][..]).into();
    println!("{:?} {:?}", from_arr, from_slice);
    let st = String::from_iter(['h', 'i']);
    println!("{}", st);
}
