// Edition 2021 disjoint field capture, capturing in loops, closures modifying
// one field while another is borrowed, captured references to array elements.
struct Point {
    x: i32,
    y: i32,
    tags: Vec<&'static str>,
}

fn main() {
    let mut p = Point { x: 1, y: 2, tags: vec!["a"] };
    // captures only p.x mutably, so p.y can still be read
    let mut bump_x = || p.x += 10;
    bump_x();
    let y_ref = &p.y;
    println!("y {}", y_ref);
    bump_x();
    println!("x {}", p.x);

    // move only the tags field
    let take_tags = move || p.tags.len();
    println!("tags {}", take_tags());
    println!("x,y still usable: {} {}", p.x, p.y);

    let mut counters = [0u32; 4];
    for i in 0..4 {
        let mut add = |k: u32| counters[i] += k * (i as u32 + 1);
        add(1);
        add(2);
    }
    println!("{:?}", counters);

    let mut total = 0;
    let values = [3, 1, 4, 1, 5];
    values.iter().for_each(|v| total += v);
    println!("total {}", total);

    let mut acc = String::new();
    let parts = ["x", "y", "z"];
    let mut i = 0;
    while i < parts.len() {
        let mut push = |s: &str| {
            acc.push_str(s);
            acc.push('.');
        };
        push(parts[i]);
        i += 1;
    }
    println!("{}", acc);

    let mut nested = (0, (0, 0));
    {
        let mut inner = || {
            nested.1 .0 += 1;
            nested.1 .1 += 2;
        };
        inner();
        inner();
    }
    println!("{:?}", nested);
    let r = &mut nested;
    let mut via_ref = || r.0 = 99;
    via_ref();
    println!("{:?}", nested);
}
