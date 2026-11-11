fn main() {
    let s = "a,b,,c,";
    let parts: Vec<&str> = s.split(',').collect();
    println!("{:?}", parts);
    println!("{:?}", s.split(',').filter(|p| !p.is_empty()).collect::<Vec<_>>());
    println!("{:?}", "  lots   of  space ".split_whitespace().collect::<Vec<_>>());
    println!("{:?}", "  lots   of  space ".split(' ').collect::<Vec<_>>());
    println!("{:?}", "k=v=w".splitn(2, '=').collect::<Vec<_>>());
    println!("{:?}", "k=v=w".rsplitn(2, '=').collect::<Vec<_>>());
    println!("{:?}", "k=v=w".split_once('='));
    println!("{:?}", "k=v=w".rsplit_once('='));
    println!("{:?}", "none".split_once('='));
    println!("{:?}", "a--b--c".split("--").collect::<Vec<_>>());
    println!("{:?}", "a1b2c3".split(|c: char| c.is_ascii_digit()).collect::<Vec<_>>());
    println!("{:?}", "x.y.z".rsplit('.').collect::<Vec<_>>());
    println!("{:?}", "a;b;c;".split_terminator(';').collect::<Vec<_>>());
    println!("{:?}", "one two".split(' ').map(|w| w.len()).collect::<Vec<_>>());
    let text = "line one\nline two\r\n\nlast";
    for (i, l) in text.lines().enumerate() {
        println!("{}: [{}]", i, l);
    }
    let csv = "name,age\nbob,30\namy,25";
    let rows: Vec<Vec<&str>> = csv.lines().map(|l| l.split(',').collect()).collect();
    println!("{:?}", rows);
    let ints: Vec<i32> = "3 -4 10 7".split(' ').map(|x| x.parse().unwrap()).collect();
    println!("{:?} {}", ints, ints.iter().sum::<i32>());
    println!("{:?}", "".split(',').collect::<Vec<_>>());
    println!("{:?}", "abc".split("").collect::<Vec<_>>());
    println!("{:?}", "AbcDef".matches(char::is_uppercase).collect::<Vec<_>>());
    println!("{:?}", "a-b_c".split(['-', '_']).collect::<Vec<_>>());
}
