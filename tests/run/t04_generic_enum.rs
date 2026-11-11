// Generic enums with methods: a hand-rolled Option and Either.
enum Maybe<T> {
    Just(T),
    Nothing,
}

impl<T: Copy> Maybe<T> {
    fn is_just(&self) -> bool {
        match self {
            Maybe::Just(_) => true,
            Maybe::Nothing => false,
        }
    }
    fn get_or(&self, d: T) -> T {
        match self {
            Maybe::Just(v) => *v,
            Maybe::Nothing => d,
        }
    }
    fn map<U>(self, f: fn(T) -> U) -> Maybe<U> {
        match self {
            Maybe::Just(v) => Maybe::Just(f(v)),
            Maybe::Nothing => Maybe::Nothing,
        }
    }
}

enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: Copy, R: Copy> Either<L, R> {
    fn flip(self) -> Either<R, L> {
        match self {
            Either::Left(l) => Either::Right(l),
            Either::Right(r) => Either::Left(r),
        }
    }
    fn left(&self) -> Maybe<L> {
        match *self {
            Either::Left(l) => Maybe::Just(l),
            Either::Right(_) => Maybe::Nothing,
        }
    }
}

enum Tree<T> {
    Leaf(T),
    Node(Box<Tree<T>>, Box<Tree<T>>),
}

fn tree_sum(t: &Tree<i64>) -> i64 {
    match t {
        Tree::Leaf(v) => *v,
        Tree::Node(l, r) => tree_sum(l) + tree_sum(r),
    }
}

fn tree_depth<T>(t: &Tree<T>) -> u32 {
    match t {
        Tree::Leaf(_) => 1,
        Tree::Node(l, r) => {
            let a = tree_depth(l);
            let b = tree_depth(r);
            1 + if a > b { a } else { b }
        }
    }
}

fn sq(x: i32) -> i32 {
    x * x
}
fn is_pos(x: i32) -> bool {
    x > 0
}

fn main() {
    let a = Maybe::Just(7);
    let b: Maybe<i32> = Maybe::Nothing;
    println!("{} {}", a.is_just(), b.is_just());
    println!("{} {}", a.get_or(0), b.get_or(-1));
    let c = a.map(sq);
    println!("{}", c.get_or(0));
    let d = Maybe::Just(-3).map(is_pos);
    println!("{}", d.get_or(true));
    let e = b.map(is_pos);
    println!("{}", e.get_or(true));

    let x: Either<i32, char> = Either::Left(10);
    let y: Either<i32, char> = Either::Right('r');
    println!("{} {}", x.left().get_or(-1), y.left().get_or(-1));
    let xf = x.flip();
    match xf {
        Either::Left(c) => println!("left {}", c),
        Either::Right(i) => println!("right {}", i),
    }
    let yf = y.flip();
    match yf {
        Either::Left(c) => println!("left {}", c),
        Either::Right(i) => println!("right {}", i),
    }

    let t = Tree::Node(
        Box::new(Tree::Leaf(1)),
        Box::new(Tree::Node(Box::new(Tree::Leaf(20)), Box::new(Tree::Node(Box::new(Tree::Leaf(300)), Box::new(Tree::Leaf(4000)))))),
    );
    println!("sum {}", tree_sum(&t));
    println!("depth {}", tree_depth(&t));
    let t2: Tree<char> = Tree::Leaf('a');
    println!("depth {}", tree_depth(&t2));
}
