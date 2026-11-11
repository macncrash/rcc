use std::fmt;

// Wrappers that forward formatting to the inner value so flags survive.
struct Id<T>(T);

impl<T: fmt::Display> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl<T: fmt::Debug> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Id<")?;
        self.0.fmt(f)?;
        f.write_str(">")
    }
}
impl<T: fmt::LowerHex> fmt::LowerHex for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

struct Sym<'a> {
    name: &'a str,
    offset: i64,
}

impl fmt::Display for Sym<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.offset {
            0 => write!(f, "{}", self.name),
            o if o > 0 => write!(f, "{}+{}", self.name, o),
            o => write!(f, "{}{}", self.name, o),
        }
    }
}

fn show(args: fmt::Arguments) -> String {
    let mut s = String::from("<<");
    fmt::write(&mut s, args).unwrap();
    s.push_str(">>");
    s
}

fn dyn_display(items: &[&dyn fmt::Display]) -> String {
    items.iter().map(|d| format!("[{:>6}]", d)).collect()
}

fn main() {
    println!("[{:>8}] [{:<8}] [{:08.2}]", Id(42), Id("s"), Id(3.14159));
    println!("{:?} {:?}", Id(1), Id("q"));
    println!("{:#?}", Id(vec![1, 2]));
    println!("{:x} {:#x} {:08x}", Id(255), Id(255), Id(255u8));
    for (n, o) in [("foo", 0), ("bar", 8), ("baz", -16)] {
        let s = Sym { name: n, offset: o };
        println!("{} [{:>10}]", s, s.to_string());
    }
    println!("{}", show(format_args!("{}-{:?}", 1, "two")));
    println!("{}", dyn_display(&[&1, &"ab", &'c', &Id(2.5), &Sym { name: "s", offset: 1 }]));
    let w: &dyn fmt::Debug = &Some(Id(0));
    println!("{:?}", w);
    let b: Box<dyn fmt::Display> = Box::new(Id(-1));
    println!("{:+}", b);
}
