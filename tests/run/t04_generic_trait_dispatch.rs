// Monomorphization of one generic fn at many types; trait with generic param
// implemented multiple times; nested generics.
trait Describe {
    fn tag() -> &'static str;
    fn val(&self) -> i64;
}
impl Describe for u8 {
    fn tag() -> &'static str {
        "u8"
    }
    fn val(&self) -> i64 {
        *self as i64
    }
}
impl Describe for i16 {
    fn tag() -> &'static str {
        "i16"
    }
    fn val(&self) -> i64 {
        *self as i64
    }
}
impl Describe for bool {
    fn tag() -> &'static str {
        "bool"
    }
    fn val(&self) -> i64 {
        *self as i64
    }
}
impl<T: Describe> Describe for Option<T> {
    fn tag() -> &'static str {
        "opt"
    }
    fn val(&self) -> i64 {
        match self {
            Some(t) => t.val() * 10,
            None => -1,
        }
    }
}
impl<A: Describe, B: Describe> Describe for (A, B) {
    fn tag() -> &'static str {
        "pair"
    }
    fn val(&self) -> i64 {
        self.0.val() + self.1.val()
    }
}

fn report<T: Describe>(x: T) {
    println!("{} {}", T::tag(), x.val());
}

fn nested<T: Describe>(x: T) -> i64 {
    fn inner<U: Describe>(u: &U) -> i64 {
        u.val() + 1
    }
    inner(&x) + inner(&Some(x))
}

struct Pipeline<A, B> {
    a: A,
    b: B,
}
impl<A: Describe, B: Describe> Pipeline<A, B> {
    fn total(&self) -> i64 {
        self.a.val() * 100 + self.b.val()
    }
    fn swap(self) -> Pipeline<B, A> {
        Pipeline { a: self.b, b: self.a }
    }
}

fn main() {
    report(7u8);
    report(-300i16);
    report(true);
    report(Some(5u8));
    report(None::<bool>);
    report((3u8, Some(-2i16)));
    report(Some((true, true)));
    println!("{}", nested(4u8));
    println!("{}", nested(false));
    let p = Pipeline { a: 1u8, b: 2i16 };
    println!("{}", p.total());
    let q = p.swap();
    println!("{}", q.total());
    let r = Pipeline { a: Some(q.a), b: (q.b, true) };
    println!("{}", r.total());
}
