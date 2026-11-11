// Utf8Error::valid_up_to / error_len follow std exactly.
fn show(b: &[u8]) {
    match std::str::from_utf8(b) {
        Ok(s) => println!("ok {:?}", s),
        Err(e) => println!("err up_to={} len={:?} | {}", e.valid_up_to(), e.error_len(), e),
    }
    println!("  lossy: {:?}", String::from_utf8_lossy(b));
}
fn main() {
    show(b"hello");
    show("h\u{e9}llo \u{20ac} \u{1F600}".as_bytes());
    show(&[b'a', 0xff, b'b']);
    show(&[b'a', 0xc3]);
    show(&[b'a', 0xc3, 0x28]);
    show(&[0xc0, 0x80]);
    show(&[0xc1, 0xbf]);
    show(&[0xe0, 0x80, 0x80]);
    show(&[0xe0, 0xa0]);
    show(&[0xe0, 0xa0, 0x41]);
    show(&[0xe2, 0x82]);
    show(&[0xe2, 0x82, 0x41, 0x42]);
    show(&[0xed, 0xa0, 0x80]);
    show(&[0xed, 0x9f, 0xbf]);
    show(&[0xf0, 0x80, 0x80, 0x80]);
    show(&[0xf0, 0x90]);
    show(&[0xf0, 0x90, 0x80]);
    show(&[0xf0, 0x90, 0x80, 0x41]);
    show(&[0xf0, 0x9f, 0x98, 0x80, 0x80]);
    show(&[0xf4, 0x90, 0x80, 0x80]);
    show(&[0xf4, 0x8f, 0xbf, 0xbf]);
    show(&[0xf5, 0x80, 0x80, 0x80]);
    show(&[0x80]);
    show(&[b'x', 0xbf, 0xbf, b'y']);
    let e = String::from_utf8(vec![b'o', b'k', 0xfe]).unwrap_err();
    println!("{} {:?}", e, e.utf8_error().error_len());
    println!("{:?}", e.into_bytes());
}
