// Hand-rolled lexer over &str bytes, no String/Vec.
fn is_ident_start(c: u8) -> bool {
    c == b'_' || (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z')
}

fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

fn lex(src: &str) {
    let b = src.as_bytes();
    let mut i = 0;
    let mut count = 0;
    while i < b.len() {
        let c = b[i];
        if c == b' ' || c == b'\n' {
            i += 1;
            continue;
        }
        let start = i;
        let kind;
        if is_ident_start(c) {
            while i < b.len() && (is_ident_start(b[i]) || is_digit(b[i])) {
                i += 1;
            }
            kind = "ident";
        } else if is_digit(c) {
            let mut v: u64 = 0;
            while i < b.len() && is_digit(b[i]) {
                v = v * 10 + (b[i] - b'0') as u64;
                i += 1;
            }
            println!("  (number value {})", v);
            kind = "number";
        } else if c == b'"' {
            i += 1;
            while i < b.len() && b[i] != b'"' {
                i += 1;
            }
            i += 1;
            kind = "string";
        } else if i + 1 < b.len() && ((c == b'=' && b[i + 1] == b'=') || (c == b'-' && b[i + 1] == b'>')) {
            i += 2;
            kind = "op2";
        } else {
            i += 1;
            kind = "punct";
        }
        count += 1;
        println!("{} {} [{}] {}..{}", count, kind, &src[start..i], start, i);
    }
    println!("tokens {}", count);
}

fn main() {
    lex("fn add(a1, b_2) -> u32 { a1 == 42 }");
    lex("let s = \"hi there\";\nx");
}
