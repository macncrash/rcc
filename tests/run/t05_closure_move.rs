// move closures: copying Copy captures, moving owned values, independence.
fn make_counter() -> impl FnMut() -> u32 {
    let mut c = 0;
    move || {
        c += 1;
        c
    }
}

fn main() {
    let x = 5;
    let add = move |y: i32| {
        // x is copied into the closure
        x + y
    };
    println!("{}", add(1));
    let x = 100; // shadow; closure keeps old copy
    println!("{} {}", add(1), x);
    let mut addp: fn(i32) -> i32 = |y| y + 1;
    println!("{}", addp(21));
    addp = |y| y * 2;
    println!("{}", addp(21));

    let s = String::from("owned");
    let consume = move || s.len();
    println!("{}", consume());
    println!("{}", consume());

    let mut n = 1;
    let mut m = move || {
        n *= 3;
        n
    };
    println!("{} {}", m(), m());
    println!("outer n {}", n);

    let mut c1 = make_counter();
    let mut c2 = make_counter();
    println!("{} {} {}", c1(), c1(), c1());
    println!("{}", c2());

    let v = vec![1, 2, 3];
    let total = move || v.iter().sum::<i32>();
    println!("{}", total());

    let mut closures: Vec<Box<dyn Fn() -> String>> = Vec::new();
    for i in 0..4 {
        let label = format!("item{}", i);
        closures.push(Box::new(move || format!("{}:{}", label, i * i)));
    }
    for c in &closures {
        println!("{}", c());
    }

    let data = [10, 20, 30];
    let get = move |i: usize| data[i];
    println!("{} {}", get(2), data[0]);
}
