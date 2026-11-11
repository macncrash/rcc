// slicing a str inside a multi-byte char panics (exit 101); range slices of arrays
fn main() {
    let s = "héllo";
    println!("len {}", s.len());
    println!("s[0..1] = {}", &s[0..1]);
    println!("s[1..3] = {}", &s[1..3]);
    let v = [1, 2, 3, 4, 5];
    let n = v.len();
    for end in 3..10 {
        let sl = &v[1..end.min(n)];
        println!("v[1..{}] = {:?}", end.min(n), sl);
    }
    let cut = s.len() - 4; // 2: inside the two-byte 'é'
    println!("about to slice at {}", cut);
    println!("{}", &s[0..cut]);
    println!("unreachable");
}
