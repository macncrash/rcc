fn main() {
    let mut v = vec![1, 2, 3, 4];
    for x in v.iter_mut() {
        *x *= 10;
    }
    println!("{:?}", v);
    for x in &mut v {
        *x += 1;
    }
    println!("{:?}", v);
    v.iter_mut().enumerate().for_each(|(i, x)| *x -= i as i32);
    println!("{:?}", v);
    let mut grid = vec![vec![0; 4]; 3];
    for (r, row) in grid.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() {
            *cell = r * 10 + c;
        }
    }
    println!("{:?}", grid);
    for row in &grid {
        let s: Vec<String> = row.iter().map(|x| x.to_string()).collect();
        println!("{}", s.join(" "));
    }
    let mut names = vec![String::from("a"), String::from("b")];
    for n in names.iter_mut() {
        n.push('!');
    }
    println!("{:?}", names);
    let total: i32 = v.iter().sum();
    let owned: Vec<i32> = v.into_iter().map(|x| x * 2).collect();
    println!("{} {:?}", total, owned);
    let mut pairs = vec![(1, 'a'), (2, 'b')];
    for (n, c) in pairs.iter_mut() {
        *n += 100;
        *c = c.to_ascii_uppercase();
    }
    println!("{:?}", pairs);
}
