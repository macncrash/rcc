#[derive(Debug, Clone, PartialEq)]
struct Rec {
    name: String,
    score: i32,
    tags: Vec<String>,
}
fn main() {
    let mut recs: Vec<Rec> = Vec::new();
    let raw = "alice:90:x,y;bob:72:z;carol:90:;dave:55:x";
    for part in raw.split(';') {
        let mut f = part.split(':');
        let name = f.next().unwrap().to_string();
        let score: i32 = f.next().unwrap().parse().unwrap();
        let tags: Vec<String> = f.next().unwrap().split(',').filter(|t| !t.is_empty()).map(|t| t.to_string()).collect();
        recs.push(Rec { name, score, tags });
    }
    for r in &recs {
        println!("{:?}", r);
    }
    recs.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
    println!("{:?}", recs.iter().map(|r| r.name.as_str()).collect::<Vec<_>>());
    let with_x: Vec<&Rec> = recs.iter().filter(|r| r.tags.iter().any(|t| t == "x")).collect();
    println!("{:?}", with_x.iter().map(|r| &r.name).collect::<Vec<_>>());
    let avg = recs.iter().map(|r| r.score).sum::<i32>() as f64 / recs.len() as f64;
    println!("avg {}", avg);
    if let Some(r) = recs.iter_mut().find(|r| r.name == "bob") {
        r.score += 10;
        r.tags.push("bumped".into());
    }
    let bob = recs.iter().find(|r| r.name == "bob").unwrap();
    println!("{:?}", bob);
    let idx = recs.iter().position(|r| r.score < 60).unwrap();
    let removed = recs.remove(idx);
    println!("removed {} left {}", removed.name, recs.len());
    let all_tags: Vec<String> = recs.iter().flat_map(|r| r.tags.clone()).collect();
    println!("{:?}", all_tags);
    let c = recs.clone();
    println!("eq {}", c == recs);
    let best = recs.iter().max_by_key(|r| (r.score, r.name.len())).unwrap();
    println!("best {}", best.name);
}
