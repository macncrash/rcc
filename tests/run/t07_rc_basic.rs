use std::rc::Rc;

#[derive(Debug)]
struct Node {
    name: String,
    val: i32,
}

fn total(xs: &[Rc<Node>]) -> i32 {
    xs.iter().map(|n| n.val).sum()
}

fn main() {
    let a = Rc::new(Node { name: "a".to_string(), val: 10 });
    println!("strong={} weak={}", Rc::strong_count(&a), Rc::weak_count(&a));
    let b = Rc::clone(&a);
    let c = a.clone();
    println!("after 2 clones strong={}", Rc::strong_count(&a));
    println!("ptr_eq a,b={} name via c={} val={}", Rc::ptr_eq(&a, &b), c.name, c.val);
    {
        let v = vec![a.clone(), b.clone(), Rc::new(Node { name: "z".into(), val: 5 })];
        println!("in scope strong={} total={}", Rc::strong_count(&a), total(&v));
        let other = &v[2];
        println!("ptr_eq a,z={} z strong={}", Rc::ptr_eq(&a, other), Rc::strong_count(other));
    }
    println!("after scope strong={}", Rc::strong_count(&a));
    drop(b);
    drop(c);
    println!("after drops strong={}", Rc::strong_count(&a));
    let n = Rc::try_unwrap(a).unwrap();
    println!("unwrapped {:?}", n);

    let mut shared = Rc::new(vec![1, 2, 3]);
    let other = shared.clone();
    Rc::make_mut(&mut shared).push(4); // clones because other exists
    println!("shared={:?} other={:?} ptr_eq={}", shared, other, Rc::ptr_eq(&shared, &other));
    Rc::make_mut(&mut shared).push(5); // unique now, no clone
    println!("shared={:?} strong={}", shared, Rc::strong_count(&shared));
    let r2 = other.clone();
    match Rc::try_unwrap(other) {
        Ok(v) => println!("unexpected unwrap {:?}", v),
        Err(back) => println!("try_unwrap failed, strong={}", Rc::strong_count(&back)),
    }
    println!("r2 strong={}", Rc::strong_count(&r2));
    let mut uniq = Rc::new(41);
    if let Some(m) = Rc::get_mut(&mut uniq) {
        *m += 1;
    }
    let u2 = uniq.clone();
    println!("get_mut while shared is_none={}", Rc::get_mut(&mut uniq).is_none());
    println!("uniq={} u2={} sum={}", uniq, u2, *uniq + *u2);
    let s: Rc<str> = Rc::from("hello rc str");
    let s2 = s.clone();
    println!("{} len={} strong={}", s2, s.len(), Rc::strong_count(&s));
    let rv: Rc<[u8]> = Rc::from(vec![9u8, 8, 7]);
    println!("{:?} {}", rv, rv.iter().map(|&x| x as u32).sum::<u32>());
}
