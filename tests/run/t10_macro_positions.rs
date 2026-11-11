// Macros invoked in expression, statement, item, pattern and type position.
macro_rules! item_const {
    ($n:ident = $v:expr) => {
        const $n: i32 = $v;
    };
}
item_const!(TEN = 10);
item_const!(TWENTY = TEN * 2);

macro_rules! ty_pair {
    ($t:ty) => { ($t, $t) };
}
macro_rules! small_pat {
    () => { 0..=9 };
}
macro_rules! tuple_pat {
    ($a:ident, $b:ident) => { ($a, $b) };
}
macro_rules! decl {
    ($n:ident = $v:expr) => {
        let $n = $v;
    };
}
macro_rules! print_stmt {
    ($($x:expr),*) => {
        $(println!("stmt: {}", $x);)*
    };
}
macro_rules! impl_hello {
    ($t:ty) => {
        impl Hello for $t {
            fn hello(&self) -> String {
                format!("hello from {}", stringify!($t))
            }
        }
    };
}
trait Hello {
    fn hello(&self) -> String;
}
struct Foo;
impl_hello!(Foo);
impl_hello!(i32);

fn swap_pair(p: ty_pair!(u8)) -> ty_pair!(u8) {
    (p.1, p.0)
}

fn classify(n: i32) -> &'static str {
    match n {
        small_pat!() => "digit",
        _ => "big",
    }
}

struct Wrapper {
    inner: ty_pair!(i64),
}

fn main() {
    println!("{} {}", TEN, TWENTY);
    println!("{:?}", swap_pair((1, 2)));
    println!("{} {} {}", classify(4), classify(10), classify(-1));
    decl!(x = 5 * 5);
    println!("x = {}", x);
    print_stmt!(1, "two", 3.5);
    let tuple_pat!(a, b) = (7, 8);
    println!("a={} b={}", a, b);
    println!("{}", Foo.hello());
    println!("{}", 5i32.hello());
    let w = Wrapper { inner: (-1, 1) };
    println!("{:?}", w.inner);
    // macro inside an expression inside a macro
    println!("{}", vec![TEN; 3].iter().sum::<i32>() + item_value());
    fn item_value() -> i32 {
        item_const!(LOCAL = 1);
        LOCAL
    }
    let arr: [ty_pair!(char); 2] = [('a', 'b'), ('c', 'd')];
    println!("{:?}", arr);
}
