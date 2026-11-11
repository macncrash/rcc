use std::collections::VecDeque;

fn main() {
    let mut d: VecDeque<i32> = VecDeque::new();
    for i in 0..5 {
        d.push_back(i);
        d.push_front(-i - 1);
    }
    println!("{:?} len={}", d, d.len());
    println!("front={:?} back={:?}", d.front(), d.back());
    println!("pop_front={:?} pop_back={:?}", d.pop_front(), d.pop_back());
    println!("d[0]={} d[3]={} get(100)={:?}", d[0], d[3], d.get(100));
    d[1] = 77;
    if let Some(x) = d.front_mut() {
        *x *= 10;
    }
    if let Some(x) = d.back_mut() {
        *x += 1000;
    }
    println!("{:?}", d);
    d.rotate_left(3);
    println!("rotate_left 3: {:?}", d);
    d.rotate_right(1);
    println!("rotate_right 1: {:?}", d);
    d.insert(2, 555);
    println!("insert: {:?}", d);
    println!("remove(4)={:?}", d.remove(4));
    d.retain(|x| x % 2 == 0);
    println!("retain even: {:?}", d);
    let v: Vec<i32> = d.iter().map(|x| x * 2).collect();
    println!("doubled vec: {:?}", v);
    for x in d.iter_mut() {
        *x -= 1;
    }
    println!("iter_mut: {:?}", d);
    d.extend([100, 200, 300]);
    println!("contains 200={} contains 7={}", d.contains(&200), d.contains(&7));
    let drained: Vec<i32> = d.drain(1..3).collect();
    println!("drained={:?} rest={:?}", drained, d);
    d.make_contiguous().sort();
    println!("sorted={:?}", d);
    let (a, b) = d.as_slices();
    println!("slices total len={}", a.len() + b.len());
    d.truncate(2);
    println!("truncate={:?}", d);
    d.swap(0, 1);
    println!("swap={:?}", d);

    // BFS over a small grid using VecDeque as a queue
    let grid = ["S..#....", ".#.#.##.", ".#...#..", ".####.#.", "......#E"];
    let h = grid.len();
    let w = grid[0].len();
    let cells: Vec<&[u8]> = grid.iter().map(|r| r.as_bytes()).collect();
    let mut dist = vec![vec![-1i32; w]; h];
    let mut q = VecDeque::new();
    dist[0][0] = 0;
    q.push_back((0usize, 0usize));
    let mut visited_order = 0;
    while let Some((r, c)) = q.pop_front() {
        visited_order += 1;
        let dirs = [(0i32, 1i32), (1, 0), (0, -1), (-1, 0)];
        for (dr, dc) in dirs {
            let nr = r as i32 + dr;
            let nc = c as i32 + dc;
            if nr < 0 || nc < 0 || nr >= h as i32 || nc >= w as i32 {
                continue;
            }
            let (nr, nc) = (nr as usize, nc as usize);
            if cells[nr][nc] == b'#' || dist[nr][nc] >= 0 {
                continue;
            }
            dist[nr][nc] = dist[r][c] + 1;
            q.push_back((nr, nc));
        }
    }
    println!("visited={} dist to E={}", visited_order, dist[h - 1][w - 1]);
    for row in &dist {
        let s: Vec<String> = row.iter().map(|d| if *d < 0 { "##".to_string() } else { format!("{:2}", d) }).collect();
        println!("{}", s.join(" "));
    }
    let from_vec: VecDeque<_> = vec![1, 2, 3].into();
    let back: Vec<_> = from_vec.into_iter().rev().collect();
    println!("{:?}", back);
}
