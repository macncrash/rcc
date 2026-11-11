use std::fmt::Write;

struct Sym {
    name: &'static str,
    addr: u64,
    size: u32,
    kind: char,
}

fn hexdump(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (row, chunk) in bytes.chunks(16).enumerate() {
        write!(out, "{:08x}  ", row * 16).unwrap();
        for i in 0..16 {
            if i < chunk.len() {
                write!(out, "{:02x} ", chunk[i]).unwrap();
            } else {
                out.push_str("   ");
            }
            if i == 7 {
                out.push(' ');
            }
        }
        out.push_str(" |");
        for &b in chunk {
            out.push(if (0x20..0x7f).contains(&b) { b as char } else { '.' });
        }
        out.push_str("|\n");
    }
    out
}

fn main() {
    let syms = [
        Sym { name: "main", addr: 0x401000, size: 120, kind: 'T' },
        Sym { name: "helper_function_long", addr: 0x401078, size: 8, kind: 't' },
        Sym { name: "counter", addr: 0x404010, size: 4, kind: 'D' },
        Sym { name: "buf", addr: 0x405000, size: 4096, kind: 'B' },
    ];
    let w = syms.iter().map(|s| s.name.len()).max().unwrap_or(4).max(4);
    println!("{:<w$}  {:>18}  {:>6}  {}", "NAME", "ADDR", "SIZE", "K", w = w);
    println!("{}", "-".repeat(w + 32));
    let mut total = 0;
    for s in &syms {
        println!("{:<w$}  {:#018x}  {:>6}  {}", s.name, s.addr, s.size, s.kind, w = w);
        total += s.size;
    }
    println!("{:>w$}  {:>18}  {:>6}", "total", "", total, w = w);
    let pct: Vec<String> = syms.iter().map(|s| format!("{:5.1}%", 100.0 * s.size as f64 / total as f64)).collect();
    println!("{}", pct.join(" "));
    let data: Vec<u8> = (0u32..40).map(|i| (i * 7 + 0x20) as u8).chain(b"\x7fELF\x00\x01".iter().copied()).collect();
    print!("{}", hexdump(&data));
    let mut bar = String::new();
    for s in &syms {
        let n = (s.size as f64).log2().round() as usize;
        writeln!(bar, "{:>8} {:#<n$}{:.<rest$}|", s.name.get(..8).unwrap_or(s.name), "", "", n = n, rest = 13 - n).unwrap();
    }
    print!("{}", bar);
}
