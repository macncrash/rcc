fn main() {
    let chars = ['a', ' ', '\n', '\t', '\r', '\'', '"', '\\', '\0', '\u{7f}', '\u{1b}', '\u{e9}', '\u{3bb}',
                 '\u{200b}', '\u{feff}', '\u{1F600}', '\u{10ffff}', '\u{301}', '\u{a0}'];
    for c in chars {
        println!("{:?} | {} | {} | {} | {}", c, c.escape_debug(), c.escape_default(), c.escape_unicode(), c as u32);
    }
    let strs = ["plain", "with \"quotes\"", "it's", "back\\slash", "tab\tnl\ncr\r", "nul\0", "\u{7f}del",
                "caf\u{e9}", "zero\u{200b}width", "e\u{301}", "emoji \u{1F600}", ""];
    for s in strs {
        println!("{:?} | {} | {}", s, s.escape_debug(), s.escape_default());
    }
    println!("{:?}", vec!["a\"b", "c'd"]);
    println!("{:?}", ('\'', "'", '"', "\""));
    println!("{:?}", Some("x\ny"));
    println!("{:?}", String::from("\u{1}\u{1f}"));
    println!("{:?}", b"\x00\x7f\xff\n");
    println!("{:?}", 'a'..='e');
    println!("{:?}", r"raw \n str");
    println!("{:?}", r#"raw "hash""#);
    println!("[{:>8?}]", 'q');
    println!("[{:<8?}]", "q");
}
