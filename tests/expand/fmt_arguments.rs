// rcc-skip: mock
// format_args! values passed to user functions taking std::fmt::Arguments
// (cannot use the mock, whose Arguments is a different type).
use std::fmt;
use std::fmt::Write as _;

fn takes(a: fmt::Arguments) -> String {
    fmt::format(a)
}

struct Sink {
    buf: String,
}
impl Sink {
    fn emit_fmt(&mut self, args: fmt::Arguments) {
        self.buf.write_fmt(args).unwrap();
        self.buf.push('\n');
    }
}

macro_rules! emit {
    ($state:expr, $($arg:tt)*) => {
        $state.emit_fmt(format_args!($($arg)*))
    };
}

fn main() {
    println!("{}", takes(format_args!("{} and {}", 1, 2)));
    println!("{}", takes(format_args!("lit only")));
    println!("{}", takes(format_args!("{1}{0}{1}", 'a', 'b')));
    let mut s = Sink { buf: String::new() };
    let reg = "rax";
    emit!(s, "    mov {}, {}", reg, 5);
    emit!(s, "    push {reg}");
    emit!(s, "    sub {dst}, {src}, t5", dst = "x1", src = "x2");
    emit!(s, "    .byte {:#04x}", 7);
    print!("{}", s.buf);
    let mut out = String::new();
    write!(&mut out, "{:>4}", 1).unwrap();
    writeln!(out, "|{}|", 2).unwrap();
    writeln!(out).unwrap();
    write!(out, "end").unwrap();
    println!("{:?}", out);
}
