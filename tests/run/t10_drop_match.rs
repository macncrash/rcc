// Drops interacting with match: by-value bindings move, `_` does not move, ref bindings,
// drops of the unmatched remainder, Result chains with `?` inside a closure.
struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

enum Msg {
    Quit,
    Text(D),
    Pair(D, D),
}

fn handle(m: Msg) {
    println!("handle start");
    match m {
        Msg::Quit => println!("quit"),
        Msg::Text(d) => println!("text {}", d.0),
        Msg::Pair(a, _) => println!("pair first {}", a.0),
    }
    println!("handle end");
}

fn inspect(m: &Msg) -> &'static str {
    match m {
        Msg::Quit => "quit",
        Msg::Text(d) => d.0,
        Msg::Pair(_, b) => b.0,
    }
}

fn chain(input: Result<D, D>) -> Result<usize, &'static str> {
    let f = |r: Result<D, D>| -> Result<usize, &'static str> {
        let _guard = D("closure-guard");
        let ok = r.map_err(|e| e.0)?;
        Ok(ok.0.len())
    };
    f(input)
}

fn main() {
    handle(Msg::Quit);
    handle(Msg::Text(D("t")));
    handle(Msg::Pair(D("p.a"), D("p.b")));
    println!("-- inspect by reference");
    let m = Msg::Pair(D("r.a"), D("r.b"));
    println!("inspect -> {}", inspect(&m));
    println!("-- wildcard does not move");
    let n = Msg::Text(D("w"));
    match n {
        _ => println!("matched wildcard"),
    }
    println!("n still owned here");
    drop(n);
    println!("-- binding @");
    let o = Some(D("at"));
    match o {
        Some(ref d @ D(name)) if name.len() == 2 => println!("bound {} {}", d.0, name),
        _ => println!("other"),
    }
    println!("-- result chains");
    println!("{:?}", chain(Ok(D("good"))));
    println!("{:?}", chain(Err(D("bad"))));
    println!("-- if let moves the binding");
    let p = Some(D("iflet-moved"));
    if let Some(d) = p {
        println!("got {}", d.0);
    }
    println!("after if let");
    println!("end of main");
}
