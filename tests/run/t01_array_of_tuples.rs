fn main() {
    let people = [("ann", 31, 5.5), ("bob", 25, 6.1), ("cy", 47, 5.9), ("di", 19, 5.2)];
    for p in people {
        println!("{} age {} height {}", p.0, p.1, p.2);
    }
    let mut oldest = people[0];
    for i in 1..people.len() {
        if people[i].1 > oldest.1 {
            oldest = people[i];
        }
    }
    println!("oldest {}", oldest.0);
    let mut total_age = 0;
    for (_, age, _) in people {
        total_age += age;
    }
    println!("total age {}", total_age);
    let mut points = [(0i32, 0i32); 6];
    for i in 0..6 {
        points[i] = (i as i32 * 3 - 7, 10 - i as i32 * i as i32);
    }
    for &(x, y) in &points {
        println!("({}, {}) manhattan {}", x, y, x.abs() + y.abs());
    }
    points[2].1 = 999;
    println!("{}", points[2].1);
    let edges = [(0usize, 1usize, 4), (1, 2, 3), (0, 2, 9), (2, 3, 1)];
    let mut dist = [1000; 4];
    dist[0] = 0;
    for _ in 0..4 {
        for &(u, v, w) in edges.iter() {
            if dist[u] + w < dist[v] {
                dist[v] = dist[u] + w;
            }
        }
    }
    for i in 0..4 {
        println!("dist[{}] = {}", i, dist[i]);
    }
}
