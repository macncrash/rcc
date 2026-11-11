use std::fmt;

struct Name(&'static str);

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // pad honours width, fill, alignment and precision
        f.pad(self.0)
    }
}

struct Money(i64);

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else if f.sign_plus() { "+" } else { "" };
        let a = self.0.unsigned_abs();
        let s = format!("{}${}.{:02}", sign, a / 100, a % 100);
        if let Some(w) = f.width() {
            write!(f, "{:>w$}", s, w = w)
        } else {
            write!(f, "{}", s)
        }
    }
}

struct Reg(u8, u8);

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "%{}", ["al", "ax", "eax", "rax"][self.1 as usize])
        } else {
            write!(f, "r{}", self.0)?;
            if let Some(p) = f.precision() {
                write!(f, ".{}", p)?;
            }
            Ok(())
        }
    }
}

struct Matrix([[i32; 3]; 2]);

impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, row) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "|")?;
            for v in row {
                write!(f, "{:>4}", v)?;
            }
            write!(f, " |")?;
        }
        Ok(())
    }
}

struct Fill;
impl fmt::Display for Fill {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "fill={:?} align={}", f.fill(), match f.align() {
            Some(fmt::Alignment::Left) => "left",
            Some(fmt::Alignment::Right) => "right",
            Some(fmt::Alignment::Center) => "center",
            None => "none",
        })?;
        write!(f, " zero={} minus={}", f.sign_aware_zero_pad(), f.sign_minus())
    }
}

fn main() {
    let n = Name("alice");
    println!("[{}] [{:10}] [{:>10}] [{:^10}] [{:*<9}] [{:.3}] [{:>8.2}]", n, n, n, n, n, n, n);
    for m in [Money(12345), Money(-5), Money(0), Money(100)] {
        println!("[{}] [{:12}] [{:+}]", m, m, m);
    }
    let r = Reg(7, 3);
    println!("{} {:#} {:.4} {}", r, r, r, Reg(0, 0));
    println!("{:#}", Reg(1, 1));
    println!("{}", Matrix([[1, -20, 300], [4, 5, 6]]));
    println!("{}", Fill);
    println!("{:_^5}", Fill);
    println!("{:<}", Fill);
    println!("{:-08}", Fill);
    let s = n.to_string() + &Money(250).to_string();
    println!("{}", s);
    let v: Vec<String> = [Money(1), Money(-199)].iter().map(|m| format!("<{:8}>", m)).collect();
    println!("{}", v.join(""));
}
