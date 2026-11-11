fn main() {
    let v: u32 = 0xdead_beef;
    println!("{:x} {:X} {:#x} {:#X}", v, v, v, v);
    println!("{:o} {:#o} {:b}", 8u8, 8u8, 5u8);
    println!("{:#b} {:#010b} {:010b}", 5u8, 5u8, 5u8);
    println!("{:08x} {:#010x} {:#06x} {:#4x}", 255, 255, 255, 255);
    println!("{:x} {:x} {:x}", -1i8, -1i32, -2i64);
    println!("{:b} {:o}", -1i8, -1i16);
    println!("{:#x}", i64::MIN);
    println!("{:x}", u128::MAX);
    println!("{:#X}", i128::MIN);
    println!("{:b}", 1u128 << 100);
    println!("{:x} {:x}", 0, 0u128);
    println!("{:#x} {:#b} {:#o}", 0, 0, 0);
    println!("{:>8x}|{:<8x}|{:^8x}|", 171, 171, 171);
    println!("{:*>#8x}|{:0>8x}|", 171, 171);
    println!("{:+x} {:+}", 10, 10);
    println!("{:x?} {:X?} {:#x?}", vec![10, 255], [171u8, 1], Some(31));
    for i in 0..18u32 {
        print!("{:x}", i);
    }
    println!();
    for shift in [0, 7, 8, 15, 31] {
        let n = 1u32 << shift;
        println!("{:>10} {:#010x} {:032b}", n, n, n);
    }
    let bytes = [0x7fu8, 0x45, 0x4c, 0x46, 2, 1, 1, 0];
    let hex: Vec<String> = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    println!("{}", hex.join(" "));
    let upper: String = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    println!("{}", upper);
    println!("{:x} {:o} {:b}", usize::MAX, u16::MAX, u8::MAX);
    println!("{:x} {:X}", 0x1234_5678_9abc_def0u64, 0xfedc_ba98u32 as i32);
}
