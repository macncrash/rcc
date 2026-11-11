// Nested modules, visibility levels, absolute and relative paths.
mod geometry {
    pub mod shapes {
        pub struct Rect {
            pub w: i32,
            pub h: i32,
            id: u32,
        }
        impl Rect {
            pub fn new(w: i32, h: i32) -> Rect {
                Rect { w, h, id: super::next_id() }
            }
            pub fn id(&self) -> u32 {
                self.id
            }
            pub fn area(&self) -> i32 {
                helpers::mul(self.w, self.h)
            }
        }
        mod helpers {
            pub fn mul(a: i32, b: i32) -> i32 {
                a * b
            }
        }
        pub(crate) fn crate_visible() -> &'static str {
            "crate-visible"
        }
        pub(super) fn parent_visible() -> &'static str {
            "parent-visible"
        }
        pub(in crate::geometry) fn in_path_visible() -> i32 {
            7
        }
    }
    static mut COUNTER: u32 = 100;
    fn next_id() -> u32 {
        unsafe {
            COUNTER += 1;
            COUNTER
        }
    }
    pub fn report() -> String {
        let a = shapes::parent_visible();
        let b = self::shapes::in_path_visible();
        format!("{} {}", a, b)
    }
    pub mod deeper {
        pub mod deepest {
            pub fn path_up() -> i32 {
                super::super::shapes::in_path_visible() * 2 + crate::top_level()
            }
        }
    }
}

fn top_level() -> i32 {
    1000
}

fn main() {
    let r = geometry::shapes::Rect::new(3, 4);
    println!("w={} h={} area={} id={}", r.w, r.h, r.area(), r.id());
    let r2 = crate::geometry::shapes::Rect::new(5, 6);
    println!("area2={} id2={}", r2.area(), r2.id());
    println!("{}", geometry::shapes::crate_visible());
    println!("{}", geometry::report());
    println!("{}", geometry::deeper::deepest::path_up());
    let r3 = ::std::boxed::Box::new(geometry::shapes::Rect::new(1, 1));
    println!("boxed id={} area={}", r3.id(), r3.area());
}
