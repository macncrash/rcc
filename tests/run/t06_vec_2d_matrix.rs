fn mul(a: &Vec<Vec<i64>>, b: &Vec<Vec<i64>>) -> Vec<Vec<i64>> {
    let n = a.len();
    let m = b[0].len();
    let k = b.len();
    let mut c = vec![vec![0; m]; n];
    for i in 0..n {
        for j in 0..m {
            for t in 0..k {
                c[i][j] += a[i][t] * b[t][j];
            }
        }
    }
    c
}
fn transpose(a: &[Vec<i64>]) -> Vec<Vec<i64>> {
    (0..a[0].len()).map(|j| a.iter().map(|row| row[j]).collect()).collect()
}
fn main() {
    let a = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let b = vec![vec![7, 8], vec![9, 10], vec![11, 12]];
    let c = mul(&a, &b);
    println!("{:?}", c);
    println!("{:?}", transpose(&a));
    let mut id = vec![vec![0i64; 4]; 4];
    for i in 0..4 {
        id[i][i] = 1;
    }
    let mut p = vec![vec![1i64, 1, 0, 0], vec![1, 0, 0, 0], vec![0, 0, 2, 1], vec![0, 0, 0, 2]];
    let mut acc = id.clone();
    for step in 0..6 {
        acc = mul(&acc, &p);
        println!("step {} {:?}", step, acc[0]);
    }
    p.swap(0, 3);
    println!("{:?}", p);
    let flat: Vec<i64> = acc.concat();
    println!("{:?}", flat);
    let rows_sum: Vec<i64> = acc.iter().map(|r| r.iter().sum()).collect();
    println!("{:?}", rows_sum);
    let mut tri: Vec<Vec<u64>> = Vec::new();
    for i in 0..8 {
        let mut row = vec![1u64; i + 1];
        for j in 1..i {
            row[j] = tri[i - 1][j - 1] + tri[i - 1][j];
        }
        tri.push(row);
    }
    for r in &tri {
        println!("{:?}", r);
    }
}
