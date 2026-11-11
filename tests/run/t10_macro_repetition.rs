// Repetition operators: $(...),* $(...)* $(...);+ $(...)? and trailing separators.
macro_rules! sum {
    ($($x:expr),*) => { 0 $(+ $x)* };
}
macro_rules! sum_trailing {
    ($($x:expr),* $(,)?) => { 0 $(+ $x)* };
}
macro_rules! product_space {
    ($($x:literal)*) => { 1 $(* $x)* };
}
macro_rules! semis {
    ($($x:expr);+) => {{
        let mut v = Vec::new();
        $(v.push($x);)+
        v
    }};
}
macro_rules! opt_default {
    ($name:ident $(= $def:expr)?) => {{
        let $name = 0 $(+ $def)?;
        $name
    }};
}
macro_rules! my_vec {
    () => { Vec::new() };
    ($($x:expr),+ $(,)?) => {{
        let mut v = Vec::new();
        $(v.push($x);)+
        v
    }};
}
macro_rules! pairs {
    ($($k:ident => $v:expr),* $(,)?) => {{
        let mut out = Vec::new();
        $(out.push((stringify!($k), $v));)*
        out
    }};
}
macro_rules! zip_two {
    ([$($a:expr),*] [$($b:expr),*]) => {
        vec![$(($a, $b)),*]
    };
}
macro_rules! plus_sep {
    ($($x:literal)|+) => {{ let mut n = 0; $(n += $x;)+ n }};
}

fn main() {
    println!("{}", sum!());
    println!("{}", sum!(1));
    println!("{}", sum!(1, 2, 3, 4));
    println!("{}", sum_trailing!(5, 6,));
    println!("{}", product_space!(2 3 4));
    println!("{:?}", semis!(1; 2 * 2; 3 * 3));
    println!("{} {}", opt_default!(a), opt_default!(b = 17));
    let e: Vec<i32> = my_vec![];
    println!("{:?} {:?} {:?}", e, my_vec![1], my_vec![1, 2, 3,]);
    println!("{:?}", pairs!(alpha => 1, beta => 2, gamma => 3,));
    println!("{:?}", zip_two!([1, 2, 3] ['a', 'b', 'c']));
    println!("{}", plus_sep!(1 | 2 | 3));
}
