use std::fmt::{self, Write};

struct Counter {
    chars: usize,
    lines: usize,
    out: String,
}

impl fmt::Write for Counter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.chars += s.chars().count();
        self.lines += s.matches('\n').count();
        self.out.push_str(&s.to_uppercase());
        Ok(())
    }
}

struct Limited {
    buf: String,
    cap: usize,
}

impl Write for Limited {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.buf.len() + s.len() > self.cap {
            return Err(fmt::Error);
        }
        self.buf.push_str(s);
        Ok(())
    }
}

fn emit_func(out: &mut String, name: &str, body: &[(&str, i64)]) -> fmt::Result {
    writeln!(out, ".globl {}", name)?;
    writeln!(out, "{}:", name)?;
    for (op, imm) in body {
        writeln!(out, "    {:<6} ${}, %rax", op, imm)?;
    }
    write!(out, "    ret")?;
    out.write_char('\n')?;
    out.write_str("# end\n")
}

fn render<W: Write>(w: &mut W, items: &[u32]) -> fmt::Result {
    for (i, it) in items.iter().enumerate() {
        if i > 0 {
            w.write_str(", ")?;
        }
        write!(w, "{}:{:03}", i, it)?;
    }
    Ok(())
}

fn main() {
    let mut s = String::new();
    emit_func(&mut s, "main", &[("movq", 1), ("addq", -42), ("imulq", 7)]).unwrap();
    print!("{}", s);
    println!("len={} lines={}", s.len(), s.lines().count());

    let mut c = Counter { chars: 0, lines: 0, out: String::new() };
    writeln!(c, "hello {}", "world").unwrap();
    write!(c, "\u{3bb} {:?} {:>4}", [1, 2], 'x').unwrap();
    c.write_fmt(format_args!("\n{}-{}\n", 1, 2)).unwrap();
    println!("chars={} lines={}", c.chars, c.lines);
    print!("{}", c.out);

    let mut l = Limited { buf: String::new(), cap: 10 };
    println!("{:?}", write!(l, "{}", "12345"));
    println!("{:?}", write!(l, "{}", "678901"));
    println!("buf={:?}", l.buf);
    let r = render(&mut l, &[1]);
    println!("{:?} buf={:?}", r, l.buf);

    let mut s2 = String::with_capacity(4);
    render(&mut s2, &[5, 50, 500, 5000]).unwrap();
    println!("{}", s2);
    let mut s3 = String::from("prefix:");
    write!(&mut s3, "{:#x}", 255).unwrap();
    println!("{}", s3);
    let e = fmt::Error;
    println!("{:?} {}", e, e);
}
