// PENDING compiler bug (found by the std::sync/thread work: thread::spawn
// first kept the closure in an `Option<F>` and the closure's captured `Arc`
// was released twice).
//
// A value of type `Option<C>` or `(C, u8)` where `C` is a closure that
// captures a non-Copy value by move is treated as `Copy` when it is used as
// an operand (MIR shows `_0 = copy _2` / `(..) { copy _2, .. }`), but drop
// elaboration still schedules `drop(_2)`. The captured String is freed while
// the returned copy still owns it (use after free / double free).
// The closure itself is correctly moved; only aggregates of it go wrong, so
// the Copy query for `Option<closure>` (through `impl<T: Copy> Copy for
// Option<T>`) / tuples of closures probably accepts `closure: Copy` without
// looking at the captures.
//
// Expected (rustc):   3 / 4 / dropped D / done
// Actual (rcc):       garbage lengths, double free or crash
struct D;
impl Drop for D {
    fn drop(&mut self) {
        println!("dropped D");
    }
}
fn wrap<F: FnOnce() -> usize>(f: F) -> Option<F> {
    let o = Some(f);
    o
}
fn wrap2<F: FnOnce() -> usize>(f: F) -> (Option<F>, u8) {
    (Some(f), 1)
}
fn main() {
    let s = String::from("abc");
    let b = wrap(move || s.len());
    println!("{}", (b.unwrap())());
    let s = String::from("abcd");
    let b = wrap2(move || s.len());
    println!("{}", (b.0.unwrap())());
    let d = D;
    let c = wrap(move || {
        let _x = &d;
        0
    });
    drop(c);
    println!("done");
}
