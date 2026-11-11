use std::borrow::Cow;

fn normalize(s: &str) -> Cow<str> {
    if s.contains(' ') || s.chars().any(|c| c.is_uppercase()) {
        Cow::Owned(s.replace(' ', "_").to_lowercase())
    } else {
        Cow::Borrowed(s)
    }
}

fn abs_all(v: &mut Cow<[i32]>) {
    for i in 0..v.len() {
        if v[i] < 0 {
            v.to_mut()[i] = -v[i];
        }
    }
}

fn kind<T: ?Sized + ToOwned>(c: &Cow<T>) -> &'static str {
    match c {
        Cow::Borrowed(_) => "borrowed",
        Cow::Owned(_) => "owned",
    }
}

fn main() {
    for s in ["hello", "Hello World", "snake_case", "a b c"] {
        let n = normalize(s);
        println!("{:?} -> {:?} ({}) len={}", s, n, kind(&n), n.len());
    }
    let data = [1, 2, 3];
    let mut c1: Cow<[i32]> = Cow::Borrowed(&data);
    abs_all(&mut c1);
    println!("{:?} {}", c1, kind(&c1));
    let data2 = [1, -2, 3, -4];
    let mut c2: Cow<[i32]> = Cow::Borrowed(&data2);
    abs_all(&mut c2);
    println!("{:?} {}", c2, kind(&c2));
    let owned: Vec<i32> = c2.into_owned();
    println!("into_owned {:?} orig {:?}", owned, data2);
    let mut s: Cow<str> = Cow::Borrowed("base");
    println!("{} {}", s, kind(&s));
    s.to_mut().push_str("+more");
    println!("{} {}", s, kind(&s));
    let joined: String = s.clone().into_owned() + "!";
    println!("{}", joined);
    let from_string: Cow<str> = String::from("own").into();
    let from_str: Cow<str> = "lit".into();
    println!("{} {} {} {}", from_string, kind(&from_string), from_str, kind(&from_str));
    let lossy = String::from_utf8_lossy(b"plain ascii");
    println!("{} {}", lossy, kind(&lossy));
    let lossy2 = String::from_utf8_lossy(b"bad \xff byte");
    println!("{} {}", lossy2, kind(&lossy2));
    let eq = Cow::Borrowed("x") == Cow::<str>::Owned("x".to_string());
    println!("eq={}", eq);
    let total: usize = ["a", "B b", "cc"].iter().map(|s| normalize(s)).map(|c| c.len()).sum();
    println!("total={}", total);
}
