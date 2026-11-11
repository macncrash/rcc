// Generic functions bounded by Fn, FnMut, FnOnce; passing closures and fn items.
fn apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 {
    f(x)
}
fn apply_mut<F: FnMut(i32)>(mut f: F, xs: &[i32]) {
    for &x in xs {
        f(x);
    }
}
fn apply_once<F: FnOnce() -> String>(f: F) -> String {
    f()
}
fn twice<F>(f: F) -> impl Fn(i32) -> i32
where
    F: Fn(i32) -> i32,
{
    move |x| f(f(x))
}
fn by_ref<F: Fn(&str) -> usize>(f: &F, s: &str) -> usize {
    f(s) + f(s)
}
fn triple(x: i32) -> i32 {
    x * 3
}

fn main() {
    println!("{}", apply(|x| x - 1, 10));
    println!("{}", apply(triple, 10));
    let k = 7;
    println!("{}", apply(|x| x * k, 6));

    let mut seen = Vec::new();
    apply_mut(|x| seen.push(x * x), &[1, 2, 3]);
    println!("{:?}", seen);
    let mut max = i32::MIN;
    apply_mut(|x| if x > max { max = x }, &[3, -9, 12, 4]);
    println!("{}", max);

    let s = String::from("moved in");
    println!("{}", apply_once(move || s + " and out"));
    let v = vec!['a', 'b'];
    println!("{}", apply_once(|| v.into_iter().collect()));
    // Fn closures are also FnOnce and FnMut
    println!("{}", apply_once(|| String::from("plain")));
    let mut acc = 0;
    apply_mut(|x| acc += x, &[5, 5]);
    println!("{}", acc);

    let t = twice(triple);
    println!("{}", t(2));
    let tt = twice(twice(|x| x + 1));
    println!("{}", tt(0));
    let f = |s: &str| s.len();
    println!("{}", by_ref(&f, "abcd"));
}
