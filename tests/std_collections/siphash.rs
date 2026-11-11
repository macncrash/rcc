// DefaultHasher (SipHash-1-3, zero keys) must hash exactly like real std.
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};

fn h<T: Hash + ?Sized>(x: &T) -> u64 {
    let mut s = DefaultHasher::new();
    x.hash(&mut s);
    s.finish()
}

#[derive(Hash)]
struct Point { x: i32, y: i32 }
#[derive(Hash)]
enum Shape { Dot, Circle(u32), Rect { w: u16, h: u16 }, Named(String, Option<u8>) }
#[derive(Hash)]
struct Unit;
#[derive(Hash)]
struct Wrapper(Vec<(u8, char)>);

fn main() {
    println!("empty finish = {:016x}", DefaultHasher::new().finish());
    println!("default finish = {:016x}", DefaultHasher::default().finish());
    println!("u8 0 = {:016x}", h(&0u8));
    println!("u8 255 = {:016x}", h(&255u8));
    println!("u16 = {:016x}", h(&0xbeefu16));
    println!("u32 = {:016x}", h(&0xdeadbeefu32));
    println!("u64 = {:016x}", h(&0x0123456789abcdefu64));
    println!("u128 = {:016x}", h(&0x0123456789abcdef_fedcba9876543210u128));
    println!("usize = {:016x}", h(&12345usize));
    println!("i8 = {:016x}", h(&-1i8));
    println!("i32 = {:016x}", h(&-123456i32));
    println!("i64 = {:016x}", h(&i64::MIN));
    println!("isize = {:016x}", h(&-7isize));
    println!("bool = {:016x} {:016x}", h(&true), h(&false));
    println!("char = {:016x} {:016x}", h(&'a'), h(&'\u{1F600}'));
    println!("unit = {:016x}", h(&()));
    for s in ["", "a", "ab", "abcdefg", "abcdefgh", "abcdefghi", "hello world", "the quick brown fox jumps over the lazy dog"] {
        println!("str {:?} = {:016x} String = {:016x}", s, h(s), h(&s.to_string()));
    }
    println!("&str ref = {:016x}", h(&"abc"));
    println!("slice u8 = {:016x}", h(&[1u8, 2, 3][..]));
    println!("array u8 = {:016x}", h(&[1u8, 2, 3]));
    println!("vec u32 = {:016x}", h(&vec![1u32, 2, 3, 4, 5]));
    println!("vec empty = {:016x}", h(&Vec::<u64>::new()));
    println!("vec string = {:016x}", h(&vec!["a".to_string(), "bc".to_string()]));
    println!("tuple1 = {:016x}", h(&(1u8,)));
    println!("tuple2 = {:016x}", h(&(1u32, "x")));
    println!("tuple3 = {:016x}", h(&(1usize, 2u64, 3i16)));
    println!("tuple5 = {:016x}", h(&(1u8, 2u16, 3u32, 4u64, 5usize)));
    println!("option = {:016x} {:016x}", h(&Some(5u32)), h(&None::<u32>));
    println!("result = {:016x} {:016x}", h(&Ok::<u8, String>(1)), h(&Err::<u8, String>("e".into())));
    println!("point = {:016x}", h(&Point { x: 1, y: -2 }));
    println!("shape dot = {:016x}", h(&Shape::Dot));
    println!("shape circle = {:016x}", h(&Shape::Circle(9)));
    println!("shape rect = {:016x}", h(&Shape::Rect { w: 3, h: 4 }));
    println!("shape named = {:016x}", h(&Shape::Named("n".into(), Some(1))));
    println!("unit struct = {:016x}", h(&Unit));
    println!("wrapper = {:016x}", h(&Wrapper(vec![(1, 'a'), (2, 'b')])));
    println!("box = {:016x}", h(&Box::new(42u64)));
    println!("rc str = {:016x}", h(&std::rc::Rc::<str>::from("rc")));
    println!("nested ref = {:016x}", h(&&&7u32));
    let bm: BTreeMap<u8, &str> = [(2, "b"), (1, "a")].into_iter().collect();
    println!("btreemap = {:016x}", h(&bm));
    let bs: BTreeSet<i64> = [3, 1, 2].into_iter().collect();
    println!("btreeset = {:016x}", h(&bs));
    let dq: VecDeque<u16> = [5, 6, 7].into_iter().collect();
    println!("vecdeque = {:016x}", h(&dq));
    // raw writes of every length 0..=17, in one and in two pieces
    let data: Vec<u8> = (0..17u8).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
    for n in 0..=data.len() {
        let mut a = DefaultHasher::new();
        a.write(&data[..n]);
        let mut b = DefaultHasher::new();
        b.write(&data[..n / 3]);
        b.write(&data[n / 3..n]);
        println!("write {:2} = {:016x} split_equal={}", n, a.finish(), a.finish() == b.finish());
    }
    // mixed integer writes (the tail buffer crosses word boundaries)
    let mut m = DefaultHasher::new();
    m.write_u8(1);
    m.write_u32(2);
    m.write_u16(3);
    m.write_u64(4);
    m.write_usize(5);
    m.write_i128(-6);
    println!("mixed = {:016x}", m.finish());
    // finish does not reset; hashing more continues
    let mut c = DefaultHasher::new();
    c.write_u32(1);
    let f1 = c.finish();
    c.write_u32(2);
    println!("continue = {:016x} {:016x}", f1, c.finish());
    // clone mid-stream
    let mut d = DefaultHasher::new();
    "abc".hash(&mut d);
    let mut e = d.clone();
    e.write_u8(9);
    d.write_u8(9);
    println!("clone equal = {}", d.finish() == e.finish());
    // BuildHasherDefault<DefaultHasher> is deterministic in real std
    let bh: BuildHasherDefault<DefaultHasher> = Default::default();
    println!("hash_one = {:016x}", bh.hash_one("key"));
    println!("hash_one tuple = {:016x}", bh.hash_one((1u32, 2u32)));
    // std::hash::DefaultHasher path
    let mut p = std::hash::DefaultHasher::new();
    42u32.hash(&mut p);
    println!("std::hash path = {:016x}", p.finish());
}
