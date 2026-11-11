// PENDING compiler bug (found by the std::collections work, t12_calc_interp
// and t12_preprocessor crash because of it).
//
// `let Some(x) = opt else { return ...; };` where `x` needs drop: on the
// `else` path (which diverges with `return`), rcc still emits the scope-end
// drop of `x`, although `x` was never initialized. The generated C runs
// `rcc_drop_N(&x)` on an uninitialized local (see the `return _0` block of
// `call` in --emit-c output), which frees garbage pointers: SIGTRAP/SIGABRT
// or silent memory corruption depending on stack contents.
//
// Expected (rustc):   none / drop D(7) / some
// Actual (rcc):       drop D(0) / none / drop D(7) / some   (D(0) is the
//                     uninitialized `x` being dropped on the else path)
struct D(u64);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop D({})", self.0);
    }
}

#[inline(never)]
fn scribble() -> [u64; 8] {
    // leave a recognisable value in the stack slot `x` will use
    let a = [0xdead_beefu64; 8];
    std::hint::black_box(a)
}

fn f(o: Option<D>) -> &'static str {
    let Some(x) = o else {
        return "none";
    };
    let _ = x.0;
    "some"
}

fn main() {
    scribble();
    println!("{}", f(None));
    let d = Some(D(7));
    println!("{}", f(d));
}
