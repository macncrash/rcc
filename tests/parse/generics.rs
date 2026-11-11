// `>>` / `>=` / `>>=` splitting, turbofish, qualified paths, bounds.
use std::collections::HashMap;
fn f<'a, T: Clone + 'a, const N: usize>(x: &'a [T; N]) -> Vec<Vec<T>>
where
    T: Into<Option<Box<dyn Fn(u8) -> u8 + Send>>>,
    for<'b> &'b T: IntoIterator<Item = &'b u8>,
{
    let v: Vec<Vec<u8>>= Vec::new();
    let m: HashMap<String, Vec<Option<u32>>> = HashMap::new();
    let mut w: Option<Option<i32>>=None;
    let s = iter.collect::<Vec<_>>();
    let n = size_of::<T>();
    let q = <T as Default>::default();
    let r = <Vec<u8>>::new();
    let z = Vec::<Vec<u8>>::with_capacity(3);
    let t: <T as IntoIterator>::Item = x;
    let g: &dyn for<'c> Fn(&'c u8) -> &'c u8 = &h;
    let fp: unsafe extern "C" fn(*const i8, ...) -> i32 = printf;
    let ar: [[u8; 4]; N * 2] = [[0; 4]; N * 2];
    let i: impl Iterator<Item = u32> + '_ = 0..3;
    let k = a < b && c > d;
    let tup: (i32,) = (1,);
    let un: () = ();
    let nv: fn() -> ! = abort;
    let p: *mut *const u8 = 0 as *mut *const u8;
    let wild: Vec<_> = vec![];
    if a >> b > c {}
    x >>= 1;
}
struct S<T: ?Sized + 'static, U = i32>(Box<T>, U);
