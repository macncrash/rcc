// PhantomData-tagged ids, zero-sized types, newtype wrappers.
use std::marker::PhantomData;

struct Id<T> {
    raw: u32,
    _m: PhantomData<T>,
}
impl<T> Id<T> {
    fn new(raw: u32) -> Self {
        Id { raw, _m: PhantomData }
    }
}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Id::new(self.raw)
    }
}
impl<T> Copy for Id<T> {}

struct Block;
struct Value;

struct Arena<T> {
    items: Vec<T>,
}
impl<T> Arena<T> {
    fn alloc(&mut self, t: T) -> Id<T> {
        self.items.push(t);
        Id::new(self.items.len() as u32 - 1)
    }
    fn get(&self, id: Id<T>) -> &T {
        &self.items[id.raw as usize]
    }
}

struct BlockData {
    name: &'static str,
}
struct ValueData {
    n: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Celsius(f64);
#[derive(Debug, Clone, Copy)]
struct Fahrenheit(f64);
impl Celsius {
    fn to_f(self) -> Fahrenheit {
        Fahrenheit(self.0 * 1.8 + 32.0)
    }
}

struct Zst;
impl Zst {
    fn hello(&self) -> &'static str {
        "zst"
    }
}

fn main() {
    let mut blocks: Arena<BlockData> = Arena { items: Vec::new() };
    let mut values: Arena<ValueData> = Arena { items: Vec::new() };
    let b0 = blocks.alloc(BlockData { name: "entry" });
    let b1 = blocks.alloc(BlockData { name: "exit" });
    let v0 = values.alloc(ValueData { n: 10 });
    let v1 = values.alloc(ValueData { n: -4 });
    let v2 = values.alloc(ValueData { n: 99 });
    let copy = v2;
    println!("{} {}", blocks.get(b0).name, blocks.get(b1).name);
    println!("{} {} {}", values.get(v0).n, values.get(v1).n, values.get(copy).n);
    println!("{} {} {}", b1.raw, v2.raw, copy.raw);
    let _tb: Id<Block> = Id::new(1);
    let _tv: Id<Value> = Id::new(2);
    println!("{}", std::mem::size_of::<Id<Block>>());
    println!("{}", std::mem::size_of::<PhantomData<String>>());
    println!("{}", std::mem::size_of::<Zst>());
    println!("{}", Zst.hello());
    let c = Celsius(100.0);
    println!("{:?} {:?} {}", c, c.to_f(), c.to_f().0);
    println!("{}", Celsius(3.0) < Celsius(4.0));
    let arr = [Zst, Zst, Zst];
    println!("{} {}", arr.len(), arr[2].hello());
}
