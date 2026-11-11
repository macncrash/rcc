fn main() {
    let v: Vec<i32> = (1..=7).collect();
    for w in v.windows(3) {
        println!("window {:?} sum {}", w, w.iter().sum::<i32>());
    }
    for c in v.chunks(3) {
        println!("chunk {:?}", c);
    }
    for c in v.chunks_exact(2) {
        println!("exact {:?}", c);
    }
    println!("remainder {:?}", v.chunks_exact(2).remainder());
    for c in v.rchunks(3) {
        println!("rchunk {:?}", c);
    }
    let mut m = vec![0u8; 10];
    for (i, c) in m.chunks_mut(4).enumerate() {
        for x in c.iter_mut() {
            *x = i as u8 + 1;
        }
    }
    println!("{:?}", m);
    let diffs: Vec<i32> = [1, 4, 9, 16, 25].windows(2).map(|w| w[1] - w[0]).collect();
    println!("diffs {:?}", diffs);
    let bytes = b"abcdefgh";
    let words: Vec<u32> = bytes
        .chunks(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    println!("{:?}", words);
    println!("count windows {}", v.windows(10).count());
    let parts: Vec<&[i32]> = v.split(|x| x % 3 == 0).collect();
    println!("split {:?}", parts);
}
