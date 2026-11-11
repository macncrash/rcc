fn main() {
    let mut v: Vec<i32> = (1..=10).collect();
    let d: Vec<i32> = v.drain(2..5).collect();
    println!("drained {:?} left {:?}", d, v);
    let d2: Vec<i32> = v.drain(..2).collect();
    println!("drained {:?} left {:?}", d2, v);
    let d3: Vec<i32> = v.drain(3..).collect();
    println!("drained {:?} left {:?}", d3, v);
    let sum: i32 = v.drain(..).sum();
    println!("sum {} left {:?} empty {}", sum, v, v.is_empty());
    let mut s: Vec<String> = vec!["x".into(), "y".into(), "z".into()];
    for item in s.drain(1..=1) {
        println!("drain item {}", item);
    }
    println!("{:?}", s);
    let mut st = String::from("hello world");
    let w: String = st.drain(..6).collect();
    println!("drained '{}' left '{}'", w, st);
    let mut v4 = vec![1, 2, 3, 4, 5];
    {
        let mut it = v4.drain(1..4);
        println!("first {:?}", it.next());
        println!("back {:?}", it.next_back());
    }
    println!("after partial drain {:?}", v4);
}
