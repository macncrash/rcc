// rcc-skip: mock
// macro_rules!: fragments, repetitions, recursion, positions, scoping.
#![allow(unused)]

// ---- ccc's five macros, verbatim ------------------------------------------
pub struct Out {
    pub s: String,
}
impl Out {
    pub fn emit_fmt(&mut self, a: std::fmt::Arguments) {
        use std::fmt::Write;
        self.s.write_fmt(a).unwrap();
        self.s.push('\n');
    }
}
pub struct State {
    pub out: Out,
}

#[macro_export]
macro_rules! emit {
    ($state:expr, $($arg:tt)*) => {
        $state.out.emit_fmt(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! delegate_to_impl {
    // Entry point: parse one delegation at a time and recurse
    () => {};

    // &mut self method with return type
    (fn $name:ident(&mut self $(, $pname:ident : $pty:ty)*) -> $ret:ty => $impl_name:ident; $($rest:tt)*) => {
        fn $name(&mut self $(, $pname : $pty)*) -> $ret { self.$impl_name($($pname),*) }
        delegate_to_impl!{ $($rest)* }
    };

    // &mut self method without return type
    (fn $name:ident(&mut self $(, $pname:ident : $pty:ty)*) => $impl_name:ident; $($rest:tt)*) => {
        fn $name(&mut self $(, $pname : $pty)*) { self.$impl_name($($pname),*) }
        delegate_to_impl!{ $($rest)* }
    };

    // &self method with return type
    (fn $name:ident(&self $(, $pname:ident : $pty:ty)*) -> $ret:ty => $impl_name:ident; $($rest:tt)*) => {
        fn $name(&self $(, $pname : $pty)*) -> $ret { self.$impl_name($($pname),*) }
        delegate_to_impl!{ $($rest)* }
    };

    // &self method without return type
    (fn $name:ident(&self $(, $pname:ident : $pty:ty)*) => $impl_name:ident; $($rest:tt)*) => {
        fn $name(&self $(, $pname : $pty)*) { self.$impl_name($($pname),*) }
        delegate_to_impl!{ $($rest)* }
    };
}

trait Backend {
    fn size(&mut self, n: i64) -> i64;
    fn note(&mut self, a: &str, b: i32);
    fn name(&self) -> &'static str;
    fn show(&self, x: u8);
}
struct Impl {
    log: Vec<String>,
}
impl Impl {
    fn size_impl(&mut self, n: i64) -> i64 {
        n * 2
    }
    fn note_impl(&mut self, a: &str, b: i32) {
        self.log.push(format!("{a}{b}"));
    }
    fn name_impl(&self) -> &'static str {
        "impl"
    }
    fn show_impl(&self, x: u8) {
        println!("show {}", x);
    }
}
impl Backend for Impl {
    delegate_to_impl! {
        fn size(&mut self, n: i64) -> i64 => size_impl;
        fn note(&mut self, a: &str, b: i32) => note_impl;
        fn name(&self) -> &'static str => name_impl;
        fn show(&self, x: u8) => show_impl;
    }
}

fn skip_contains(skip: &[&str], s: &str) -> bool {
    skip.contains(&s)
}
fn eliminate_a(v: &mut Vec<i32>, k: i32) -> bool {
    v.push(k);
    k > 1
}
fn eliminate_b(v: &mut Vec<i32>) -> bool {
    v.pop();
    false
}

fn passes() {
    let time_passes = false;
    let iterations = 2;
    let mut prev_pass_changes = [1usize, 0, 3, 0, 5, 0, 0, 0, 0, 2];
    for iter in 0..iterations {
        macro_rules! timed_pass {
            ($name:expr, $body:expr) => {{
                if time_passes {
                    let t0 = std::time::Instant::now();
                    let n = $body;
                    let elapsed = t0.elapsed().as_secs_f64();
                    eprintln!("[PASS] iter={} {}: {:.4}s ({} changes)", iter, $name, elapsed, n);
                    n
                } else {
                    $body
                }
            }};
        }
        macro_rules! should_run {
            ($self_idx:expr, $($upstream:expr),*) => {{
                prev_pass_changes[$self_idx] > 0 $(|| prev_pass_changes[$upstream] > 0)*
            }};
        }
        if should_run!(0, 4, 9) {
            let n = timed_pass!("cfg_simplify1", iter * 10 + 1);
            println!("ran cfg_simplify1 {}", n);
        }
        if should_run!(1, 3, 5) {
            println!("ran copy_prop");
        }
        if should_run!(7, 8) {
            println!("ran 7");
        }
        prev_pass_changes[0] = 0;
        prev_pass_changes[4] = 0;
    }
    let skip: Vec<&str> = vec!["eliminate_b"];
    let mut lines = vec![1, 2];
    macro_rules! pass {
        ($name:ident($($arg:expr),*)) => {
            !skip.contains(&stringify!($name)) && $name($($arg),*)
        };
    }
    let mut changed2 = false;
    changed2 |= pass!(eliminate_a(&mut lines, 3));
    changed2 |= pass!(eliminate_b(&mut lines));
    println!("{:?} {}", lines, changed2);
}

// ---- fragments ------------------------------------------------------------
macro_rules! frags {
    (expr $e:expr) => { println!("expr {}", $e) };
    (ident $i:ident) => { println!("ident {}", stringify!($i)) };
    (ty $t:ty) => { println!("ty {}", std::mem::size_of::<$t>()) };
    (tt $($t:tt)*) => { println!("tt {}", stringify!($($t)*)) };
    (pat $p:pat) => { println!("pat {}", matches!(Some(3), $p)) };
    (pat_param $p:pat_param | $q:pat_param) => { println!("pat_param {}", matches!(4, $p | $q)) };
    (literal $l:literal) => { println!("literal {}", $l) };
    (block $b:block) => { println!("block {}", $b) };
    (stmt $s:stmt) => {{ $s; println!("stmt") }};
    (item $i:item) => {{ $i println!("item {}", made()) }};
    (path $p:path) => { println!("path {}", <$p>::default()) };
    (lifetime $l:lifetime) => { let _r: &$l str = "x"; println!("lifetime") };
    (vis $v:vis struct) => { $v struct Hidden; println!("vis") };
    (meta $m:meta) => { #[$m] fn metaf() {} println!("meta") };
}

// ---- repetitions ------------------------------------------------------------
macro_rules! sum {
    () => { 0 };
    ($x:expr $(, $rest:expr)* $(,)?) => { $x + sum!($($rest),*) };
}
macro_rules! nested {
    ($( $name:ident : [ $($v:expr),* ] );* $(;)?) => {
        $( println!("{} {:?} {}", stringify!($name), [0i32 $(, $v)*], 0 $(+ $v)*); )*
    };
}
macro_rules! plus_rep {
    ($($x:expr)+) => { [$($x * 2),+] };
}
macro_rules! opt {
    ($a:expr $(=> $b:expr)?) => { $a $(+ $b)? };
}
macro_rules! kv_map {
    ($($k:expr => $v:expr),* $(,)?) => {{
        let mut m = std::collections::BTreeMap::new();
        $( m.insert($k, $v); )*
        m
    }};
}
// separator other than comma, and nested depth with outer var reuse
macro_rules! cross {
    ($($a:ident)|* ; $($b:literal)*) => {
        (vec![$(stringify!($a)),*], vec![$($b),*])
    };
}

// ---- positions ----------------------------------------------------------------
macro_rules! make_struct {
    ($name:ident { $($f:ident : $t:ty),* }) => {
        #[derive(Debug, Default, Clone, PartialEq)]
        struct $name { $($f: $t),* }
        impl $name {
            fn fields() -> &'static [&'static str] { &[$(stringify!($f)),*] }
        }
    };
}
make_struct!(Gen { a: i32, b: String });

macro_rules! my_ty {
    () => { Vec<u32> };
}
macro_rules! my_pat {
    ($x:ident) => { Some($x) | Option::Some($x) };
}
macro_rules! some_pat {
    ($x:pat) => { Some($x) };
}
macro_rules! stmts {
    ($v:ident) => {
        let mut $v = 1;
        $v += 1;
        $v *= 10;
    };
}

// ---- macros defining macros, $crate, recursion (tt muncher) ---------------
macro_rules! make_adder {
    ($name:ident, $n:expr) => {
        macro_rules! $name {
            ($x:expr) => { $x + $n };
        }
    };
}
make_adder!(add5, 5);

mod util {
    pub fn helper() -> i32 {
        7
    }
    #[macro_export]
    macro_rules! call_helper {
        () => { $crate::util::helper() };
    }
}

macro_rules! count_tts {
    () => { 0usize };
    ($odd:tt $($a:tt $b:tt)*) => { (count_tts!($($a)*) << 1) | 1 };
    ($($a:tt $even:tt)*) => { count_tts!($($a)*) << 1 };
}

macro_rules! reverse {
    ([] $($rev:expr),*) => { [$($rev),*] };
    ([$first:expr $(, $rest:expr)*] $($rev:expr),*) => { reverse!([$($rest),*] $first $(, $rev)*) };
}

// textual scoping: a later definition shadows an earlier one
macro_rules! which {
    () => { "first" };
}
fn use_first() -> &'static str {
    which!()
}
macro_rules! which {
    () => { "second" };
}

// hygiene of locals
macro_rules! using_a {
    ($e:expr) => {{
        let a = 42;
        $e + a
    }};
}
macro_rules! swap {
    ($a:ident, $b:ident) => {
        let tmp = $a;
        $a = $b;
        $b = tmp;
    };
}

// implicit format captures refer to the macro's own locals
macro_rules! show_cap {
    ($x:expr) => {{
        let v = $x * 2;
        let w = 1;
        println!("{v} {w:>3} {}", v);
        format!("{v}{{v}}")
    }};
}

// different delimiters for invocation; trailing semicolon in expr position
macro_rules! three {
    () => { 3 };
}

fn main() {
    let mut st = State { out: Out { s: String::new() } };
    emit!(st, "    mov {}, {}", "rax", 1);
    emit!(st, "    ret");
    crate::emit!(st, "    nop {x}", x = 2);
    print!("{}", st.out.s);
    let mut im = Impl { log: vec![] };
    println!("{} {}", im.size(21), im.name());
    im.note("n", 1);
    im.show(9);
    println!("{:?}", im.log);
    passes();

    frags!(expr 1 + 2 * 3);
    frags!(ident hello);
    frags!(ty (u8, u32));
    frags!(tt a + b, [c] { d });
    frags!(pat Some(1..=5));
    frags!(pat_param 4 | 5);
    frags!(literal -7);
    frags!(literal "s");
    frags!(block { let q = 2; q * q });
    frags!(stmt let _z = 5);
    frags!(item fn made() -> i32 { 11 });
    frags!(path String);
    frags!(lifetime 'static);
    frags!(vis pub(crate) struct);
    frags!(meta allow(dead_code));

    println!("{}", sum!(1, 2, 3, 4,));
    println!("{}", sum!());
    nested!(a: [1, 2]; b: []; c: [3];);
    println!("{:?}", plus_rep!(1 2 3));
    println!("{} {}", opt!(1), opt!(1 => 2));
    println!("{:?}", kv_map! { "x" => 1, "y" => 2 });
    println!("{:?}", cross!(p | q ; 1 2));

    let g = Gen { a: 1, b: "s".into() };
    println!("{:?} {:?} {}", g, Gen::fields(), g == g.clone());
    let v: my_ty!() = vec![1, 2];
    println!("{:?}", v);
    match Some(3) {
        my_pat!(x) if x > 100 => println!("never {}", x),
        some_pat!(y) if y > 2 => println!("some_pat {}", y),
        _ => println!("other"),
    }
    stmts!(w);
    println!("w = {}", w);
    println!("{}", add5!(10));
    println!("{}", call_helper!());
    println!("{}", count_tts!(a b c d e f g));
    println!("{:?}", reverse!([1, 2, 3, 4]));
    println!("{} {}", use_first(), which!());
    let a = 10;
    println!("hyg {}", using_a!(a * 1));
    let mut x = 1;
    let mut tmp = 99;
    swap!(x, tmp);
    println!("x={} tmp={}", x, tmp);
    let v = 100;
    let w = 7;
    println!("{}", show_cap!(v + 1));
    println!("{v} {w}");
    let t1 = three![];
    let t2 = three! {};
    println!("{}", t1 + t2 + three!());
}
