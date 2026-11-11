fn main() {
    let path = "src/backend/x86/codegen.rs";
    match path.rfind('/') {
        Some(i) => println!("dir [{}] file [{}]", &path[..i], &path[i + 1..]),
        None => println!("no slash"),
    }
    match path.strip_suffix(".rs") {
        Some(stem) => println!("stem [{}]", stem),
        None => println!("not rs"),
    }
    println!("{}", path.strip_suffix(".c").is_none());
    let mut segs = 0;
    for seg in path.split('/') {
        segs += 1;
        println!("seg {} [{}] len {}", segs, seg, seg.len());
    }
    let line = "   #define   FOO   1   ";
    let t = line.trim();
    println!("[{}]", t);
    if let Some(rest) = t.strip_prefix('#') {
        let rest = rest.trim_start();
        println!("directive [{}]", rest);
        if let Some(after) = rest.strip_prefix("define") {
            let mut n = 0;
            for w in after.split_whitespace() {
                n += 1;
                println!("  word {} [{}]", n, w);
            }
        }
    }
    println!("[{}]", "xxhixx".trim_matches('x'));
    println!("[{}]", "--a--".trim_start_matches('-'));
    println!("[{}]", "--a--".trim_end_matches('-'));
    let s = "abcabc";
    println!("{} {}", s.find("ca").unwrap(), s.rfind("bc").unwrap());
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        print!("{}", bytes[i] - b'a');
        i += 1;
    }
    println!();
    let mut ci = 0;
    for (idx, c) in "añb".char_indices() {
        println!("char {} at byte {} ({})", c, idx, ci);
        ci += 1;
    }
    println!("{}", "Hello".eq_ignore_ascii_case("hELLO"));
    println!("{}", "abc".is_char_boundary(1));
    let mut cnt = 0;
    for _ in "a,b,,c".split(',') {
        cnt += 1;
    }
    println!("pieces {}", cnt);
    let (a, b) = "key: value".split_at(3);
    println!("[{}] [{}]", a, b);
    let empty = "";
    println!("{} {}", empty.chars().next().is_none(), empty.as_bytes().len());
    println!("{}", std::str::from_utf8(b"bytes ok").unwrap());
    println!("{}", { let bad = [b"ok"[0], 0xff, 0xfe]; std::str::from_utf8(&bad).is_err() });
}
