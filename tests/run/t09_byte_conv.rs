// to_le_bytes / to_be_bytes / from_le_bytes / from_be_bytes
fn print_bytes(label: &str, b: &[u8]) {
    print!("{}:", label);
    for x in b {
        print!(" {:02x}", x);
    }
    println!();
}

fn main() {
    let a: u16 = 0xBEEF;
    print_bytes("u16 le", &a.to_le_bytes());
    print_bytes("u16 be", &a.to_be_bytes());
    let b: u32 = 0xDEAD_BEEF;
    print_bytes("u32 le", &b.to_le_bytes());
    print_bytes("u32 be", &b.to_be_bytes());
    let c: i64 = -2;
    print_bytes("i64 le", &c.to_le_bytes());
    print_bytes("i64 be", &c.to_be_bytes());
    let d: u128 = 0x0102_0304_0506_0708_090a_0b0c_0d0e_0f10;
    print_bytes("u128 le", &d.to_le_bytes());
    print_bytes("u128 be", &d.to_be_bytes());
    let f: f64 = 1.5;
    print_bytes("f64 le", &f.to_le_bytes());
    let g: f32 = -0.25;
    print_bytes("f32 be", &g.to_be_bytes());

    let bytes: [u8; 8] = [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11];
    println!("u64 from le {:x}", u64::from_le_bytes(bytes));
    println!("u64 from be {:x}", u64::from_be_bytes(bytes));
    println!("i64 from le {}", i64::from_le_bytes(bytes));
    let mut four = [0u8; 4];
    four.copy_from_slice(&bytes[2..6]);
    println!("u32 from le {:x} be {:x}", u32::from_le_bytes(four), u32::from_be_bytes(four));
    println!("i32 from be {}", i32::from_be_bytes([0xff, 0xff, 0xff, 0xfe]));
    println!("u16 from le {}", u16::from_le_bytes([0x34, 0x12]));
    println!("f64 from le {}", f64::from_le_bytes([0, 0, 0, 0, 0, 0, 0xf0, 0x3f]));
    println!("f32 from be {}", f32::from_be_bytes([0x40, 0x49, 0x0f, 0xdb]));

    // encode a little record and decode it again
    let mut buf: Vec<u8> = Vec::new();
    for v in [1u32, 256, 65536, 0xFFFF_FFFF] {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    buf.extend_from_slice(&(-1234567i32).to_be_bytes());
    print_bytes("record", &buf);
    let mut i = 0;
    while i + 4 <= 16 {
        let w = u32::from_le_bytes([buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]);
        println!("decoded {}", w);
        i += 4;
    }
    let t = i32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]);
    println!("decoded tail {}", t);
}
