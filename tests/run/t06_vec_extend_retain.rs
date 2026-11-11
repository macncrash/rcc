fn main() {
    let mut v = vec![1, 2, 3];
    v.extend(vec![4, 5]);
    println!("{:?}", v);
    v.extend([6, 7].iter());
    println!("{:?}", v);
    v.extend_from_slice(&[8, 9, 10]);
    println!("{:?}", v);
    v.extend((11..14).map(|x| x * 10));
    println!("{:?}", v);
    v.retain(|&x| x % 2 == 0);
    println!("retain even {:?}", v);
    let mut count = 0;
    v.retain(|x| {
        count += 1;
        *x < 100
    });
    println!("retain <100 {:?} visited {}", v, count);
    let mut words: Vec<String> = "apple banana cherry date elderberry fig"
        .split(' ')
        .map(String::from)
        .collect();
    words.retain(|w| w.len() > 4);
    println!("{:?}", words);
    let mut m = vec![1, 2, 3, 4, 5, 6];
    m.retain_mut(|x| {
        *x *= 3;
        *x % 2 == 0
    });
    println!("retain_mut {:?}", m);
    let mut s = String::from("ab");
    s.extend(['c', 'd']);
    s.extend(vec!["ef", "gh"]);
    println!("{}", s);
}
