// Impl blocks for one type spread across modules; traits imported to use methods.
mod types {
    #[derive(Debug, Clone)]
    pub struct Reg {
        pub num: u8,
        pub(crate) name: &'static str,
    }
    impl Reg {
        pub fn new(num: u8, name: &'static str) -> Self {
            Reg { num, name }
        }
    }
}

mod encode {
    use crate::types::Reg;
    impl Reg {
        pub fn encoding(&self) -> u8 {
            self.num & 7
        }
        pub fn needs_rex(&self) -> bool {
            self.num >= 8
        }
    }
}

mod display {
    pub trait Pretty {
        fn pretty(&self) -> String;
        fn shout(&self) -> String {
            self.pretty().to_uppercase()
        }
    }
    impl Pretty for crate::types::Reg {
        fn pretty(&self) -> String {
            format!("%{}", self.name)
        }
    }
    impl Pretty for u8 {
        fn pretty(&self) -> String {
            format!("byte {}", self)
        }
    }
}

mod user {
    // trait methods need the trait in scope
    use crate::display::Pretty;
    pub fn show(r: &crate::types::Reg) -> String {
        format!("{} / {} / {}", r.pretty(), r.shout(), r.encoding().pretty())
    }
}

fn main() {
    let regs = [
        types::Reg::new(0, "rax"),
        types::Reg::new(3, "rbx"),
        types::Reg::new(9, "r9"),
        types::Reg::new(15, "r15"),
    ];
    for r in regs.iter() {
        println!("{:?} enc={} rex={} | {}", r, r.encoding(), r.needs_rex(), user::show(r));
    }
    use display::Pretty as _;
    println!("{}", 200u8.shout());
}
