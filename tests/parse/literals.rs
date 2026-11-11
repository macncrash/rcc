// Lexer: literals, lifetimes vs chars, comments, raw identifiers.
/* block /* nested */ comment */
/// doc comment
//! inner doc (dropped)
/** block doc */
fn lits<'a, 'b: 'a>(s: &'a str) -> &'b str {
    let ints = (0, 123, 1_000_000, 0xff, 0xFF_u8, 0o777, 0b1010_1010, 255u8, 7i64, 3usize, 0x1f32);
    let big = 340282366920938463463374607431768211455u128;
    let floats = (1.0, 2.5e10, 1e-3, 3.0f32, 1f64, 0.5E+2, 1_000.000_1, 2.);
    let range = 1..2;
    let method = 1.max(2);
    let strs = ("plain", "esc \n\t\r\\\0\'\"", "\x41\u{1F600}\u{e9}", "line \
                 continued");
    let raw = (r"C:\path", r#"has "quotes""#, r##"has "# inside"##);
    let bytes = (b"bytes\xff\x00", br"raw\bytes", br#"x"#);
    let cs = (c"cstr", cr"raw c");
    let chars = ('a', '\n', '\'', '\\', '\x7f', '\u{10FFFF}', 'é', '中');
    let bs = (b'a', b'\xff', b'\'', b'\n');
    let lt: &'static str = "x";
    let r#match = r#fn + r#type;
    'label: loop { break 'label; }
    let tuple_idx = t.0.1;
    s
}
