fn main() {
    let text = "the cat and the hat and the bat sat on the mat";
    let mut words: Vec<&str> = text.split(' ').collect();
    println!("{} words", words.len());
    words.sort_unstable();
    println!("{:?}", words);
    let mut uniq = words.clone();
    uniq.dedup();
    println!("{:?}", uniq);
    let mut freq: Vec<(&str, usize)> = Vec::new();
    for w in &words {
        match freq.last_mut() {
            Some((lw, n)) if lw == w => *n += 1,
            _ => freq.push((w, 1)),
        }
    }
    freq.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("{:?}", freq);
    let lens: Vec<usize> = uniq.iter().map(|w| w.len()).collect();
    println!("{:?}", lens);
    let rhymes: Vec<&&str> = uniq.iter().filter(|w| w.ends_with("at")).collect();
    println!("{:?}", rhymes);
    let longest = uniq.iter().copied().max_by_key(|w| w.len()).unwrap();
    println!("{}", longest);
    let initials: String = uniq.iter().filter_map(|w| w.chars().next()).collect();
    println!("{}", initials);
    let pos = uniq.binary_search(&"hat");
    println!("{:?} {:?}", pos, uniq.binary_search(&"dog"));
}
