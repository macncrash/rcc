fn main() {
    let mut v = vec![1, 2, 3, 4, 5, 6];
    let (a, b) = v.split_at(2);
    println!("{:?} {:?}", a, b);
    {
        let (l, r) = v.split_at_mut(3);
        l[0] = 100;
        r[0] = 400;
        println!("{:?} {:?}", l, r);
    }
    println!("{:?}", v);
    v.reverse();
    println!("rev {:?}", v);
    println!("first {:?} last {:?}", v.first(), v.last());
    if let Some(f) = v.first_mut() {
        *f += 1;
    }
    if let Some(l) = v.last_mut() {
        *l *= 2;
    }
    println!("{:?}", v);
    println!("get 2 {:?} get 9 {:?}", v.get(2), v.get(9));
    if let Some(x) = v.get_mut(1) {
        *x = -5;
    }
    println!("{:?}", v);
    println!("contains 400 {} contains 7 {}", v.contains(&400), v.contains(&7));
    let (head, tail) = v.split_first().unwrap();
    println!("head {} tail {:?}", head, tail);
    let (lst, init) = v.split_last().unwrap();
    println!("last {} init {:?}", lst, init);
    println!("starts {} ends {}", v.starts_with(&[401]), v.ends_with(&[1]));
    v.rotate_left(2);
    println!("rotl {:?}", v);
    v.rotate_right(1);
    println!("rotr {:?}", v);
    v.fill(9);
    println!("fill {:?}", v);
    let mut c = vec![0; 3];
    c.copy_from_slice(&[7, 8, 9]);
    println!("{:?}", c);
    let s = &c[1..];
    println!("slice {:?} to_vec {:?}", s, s.to_vec());
    let e: &[i32] = &[];
    println!("empty first {:?} split_first {:?}", e.first(), e.split_first());
    let mut z = vec![3, 1, 2];
    z.swap(0, 2);
    println!("{:?} max {:?} min {:?}", z, z.iter().max(), z.iter().min());
}
