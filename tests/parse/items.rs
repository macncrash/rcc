#![allow(dead_code)]
#![cfg_attr(test, allow(unused))]
//! Every item kind.
extern crate std as mystd;
mod external;
pub(crate) mod inline {
    #![allow(unused)]
    pub(super) fn f() {}
    pub(in crate::inline) struct Hidden;
    pub(self) const C: u8 = 1;
}
use std::{self, fmt::{self, Display as Disp}, io::*};
use super::*;
use crate::a::b as _;
use ::core::mem;
pub use self::inline::f;
use {a, b::c};

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct Named<'a, T> where T: 'a { pub a: &'a T, pub(crate) b: u32, c: [u8; 4] }
struct Tuple(pub u8, #[allow(x)] String);
struct Unit;
struct Gen<T>(T) where T: Copy;
union U { i: u32, f: f32 }
enum E<T> {
    A,
    B(T, u8),
    C { x: i32, y: Box<E<T>> },
    D = 5,
    #[cfg(test)] Ee,
}
#[repr(u8)] enum Disc { X = 1, Y = 1 << 2, Z = -1 }
pub trait Tr<T>: Clone + Send + 'static where T: Default {
    type Assoc: Iterator<Item = T> + ?Sized;
    type WithDefault = u8;
    const N: usize;
    const M: usize = 3;
    fn req(&self, x: T) -> Self::Assoc;
    fn provided(&mut self) -> usize { Self::N }
    fn generic<U: Into<T>>(self, u: U) where U: Clone;
    fn by_box(self: Box<Self>);
    fn with_lt<'a>(&'a self) -> &'a str;
    fn mut_lt<'a>(&'a mut self);
    fn mut_val(mut self) {}
    fn no_self() -> Self where Self: Sized;
    unsafe fn uns(&self);
}
unsafe trait UnsafeTr {}
impl<T> Named<'_, T> { pub fn new() -> Self { todo!() } }
impl<'a, T: Clone> Tr<T> for Named<'a, T> where T: Default {
    type Assoc = std::vec::IntoIter<T>;
    const N: usize = 1;
    fn req(&self, x: T) -> Self::Assoc { vec![x].into_iter() }
}
unsafe impl Send for U {}
impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "unit") }
}
impl<T: ?Sized> Tr2 for Box<T> {}
impl dyn Tr<u8> {}
impl<const N: usize> Arr<N> {}
type Alias<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub const MAX: u32 = u32::MAX;
const _: () = ();
static GREETING: &str = "hi";
static mut COUNTER: usize = 0;
pub const fn cf(x: u32) -> u32 { x }
pub unsafe fn uf() {}
pub extern "C" fn ef(x: i32) -> i32 { x }
pub unsafe extern "C" fn uef() {}
extern "C" {
    fn printf(fmt: *const i8, ...) -> i32;
    static errno: i32;
    pub fn malloc(n: usize) -> *mut u8;
}
extern { fn old(); }
fn impl_trait(x: impl Fn(u8) -> u8, y: &mut impl Iterator<Item = u32>) -> impl Iterator<Item = u8> + '_ { x }
fn never() -> ! { loop {} }
macro_rules! square {
    ($x:expr) => { $x * $x };
    ($($t:tt)*) => {};
}
thread_local! { static TL: u32 = 0; }
square!(3);
#[cfg(test)]
mod tests {
    #[test]
    fn t() { assert_eq!(1, 1); }
}
