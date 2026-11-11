// Module loading: foo.rs, foo/mod.rs, nested non-mod.rs dirs, #[path],
// inline modules containing file modules, inner #![cfg], #[macro_use],
// #[macro_export] used by path before its definition, include_str!.
#![allow(dead_code)]

#[macro_use]
mod macros;

mod a;
mod c;
#[path = "other/renamed.rs"]
mod x;
mod inl {
    pub mod deep;
    pub fn here() -> &'static str {
        "inl::here"
    }
}
mod gated;
#[cfg(test)]
mod tests;

fn main() {
    println!("{}", a::name());
    println!("{}", a::b::name());
    println!("{}", c::name());
    println!("{}", c::d::name());
    println!("{}", x::name());
    println!("{} {}", inl::here(), inl::deep::name());
    println!("{}", twice!(21));
    println!("{}", crate::late_export!());
    println!("{}", a::uses_exported());
    println!("{:?}", include_str!("data.txt"));
    println!("{:?}", include_bytes!("data.txt").len());
    println!("{}", module_path!());
    println!("{}", a::b::path());
}

mod later {
    #[macro_export]
    macro_rules! late_export {
        () => { "late export" };
    }
}
