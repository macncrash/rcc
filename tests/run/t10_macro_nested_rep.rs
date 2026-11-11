// Nested repetitions, repetition of groups, mixing depths.
macro_rules! matrix {
    ($([$($x:expr),*]),*) => {
        vec![$(vec![$($x),*]),*]
    };
}

macro_rules! enum_with_names {
    ($name:ident { $($var:ident),* $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum $name { $($var),* }
        impl $name {
            fn all() -> Vec<$name> {
                vec![$($name::$var),*]
            }
            fn name(self) -> &'static str {
                match self {
                    $($name::$var => stringify!($var)),*
                }
            }
        }
    };
}
enum_with_names!(Op { Add, Sub, Mul, Div });

macro_rules! table {
    ($($row:ident : $($cell:expr),+);* $(;)?) => {{
        let mut lines = Vec::new();
        $(
            let mut cells: Vec<String> = Vec::new();
            $(cells.push(format!("{}", $cell));)+
            lines.push(format!("{}=[{}]", stringify!($row), cells.join("|")));
        )*
        lines
    }};
}

// outer variable used inside inner repetition
macro_rules! prefix_all {
    ($p:expr; $($group:ident: [$($item:expr),*])*) => {{
        let mut v: Vec<String> = Vec::new();
        $(
            $(v.push(format!("{}{}{}", $p, stringify!($group), $item));)*
        )*
        v
    }};
}

macro_rules! struct_fns {
    ($($s:ident { $($f:ident : $t:ty = $d:expr),* })*) => {
        $(
            #[derive(Debug)]
            struct $s { $($f: $t),* }
            impl $s {
                fn make() -> $s { $s { $($f: $d),* } }
            }
        )*
    };
}
struct_fns! {
    A { x: i32 = 1, y: i32 = 2 }
    B { name: &'static str = "bee" }
    C { }
}

fn main() {
    let m = matrix!([1, 2, 3], [4, 5, 6], [7]);
    println!("{:?}", m);
    let empty: Vec<Vec<i32>> = matrix!();
    println!("{:?}", empty);
    for op in Op::all() {
        println!("{:?} {}", op, op.name());
    }
    println!("{}", Op::Mul == Op::all()[2]);
    for line in table!(r1: 1, 2, 3; r2: "a", "b"; r3: 4.5;) {
        println!("{}", line);
    }
    println!("{:?}", prefix_all!("#"; x: [1, 2] y: [3] z: []));
    println!("{:?} {:?} {:?}", A::make(), B::make(), C::make());
    let a = A::make();
    println!("{} {}", a.x + a.y, B::make().name);
}
