use std::collections::VecDeque;

fn sliding_max(xs: &[i32], k: usize) -> Vec<i32> {
    let mut dq: VecDeque<usize> = VecDeque::new();
    let mut out = vec![];
    for i in 0..xs.len() {
        while let Some(&b) = dq.back() {
            if xs[b] <= xs[i] { dq.pop_back(); } else { break; }
        }
        dq.push_back(i);
        if dq[0] + k <= i {
            dq.pop_front();
        }
        if i + 1 >= k {
            out.push(xs[*dq.front().unwrap()]);
        }
    }
    out
}

struct Ring {
    buf: VecDeque<u8>,
    cap: usize,
    dropped: usize,
}

impl Ring {
    fn write(&mut self, data: &[u8]) {
        for &b in data {
            if self.buf.len() == self.cap {
                self.buf.pop_front();
                self.dropped += 1;
            }
            self.buf.push_back(b);
        }
    }
    fn read(&mut self, n: usize) -> Vec<u8> {
        let n = n.min(self.buf.len());
        self.buf.drain(..n).collect()
    }
}

fn main() {
    let xs = [1, 3, -1, -3, 5, 3, 6, 7, 2, 2, 9, 0];
    println!("{:?}", sliding_max(&xs, 3));
    println!("{:?}", sliding_max(&xs, 1));
    println!("{:?}", sliding_max(&xs, 12));
    let mut r = Ring { buf: VecDeque::with_capacity(4), cap: 5, dropped: 0 };
    r.write(b"abc");
    println!("{:?}", String::from_utf8(r.read(2)).unwrap());
    r.write(b"defghij");
    println!("len={} dropped={}", r.buf.len(), r.dropped);
    println!("{:?}", String::from_utf8(r.read(10)).unwrap());
    r.write(b"xy");
    let (a, b) = r.buf.as_slices();
    println!("slices sum={}", a.len() + b.len());
    let joined: Vec<u8> = r.buf.iter().rev().copied().collect();
    println!("{:?}", joined);
    let mut d: VecDeque<i32> = (1..=8).collect();
    let mut josephus = vec![];
    while !d.is_empty() {
        d.rotate_left(2 % d.len());
        josephus.push(d.pop_front().unwrap());
    }
    println!("josephus={:?}", josephus);
    let mut d2: VecDeque<i32> = VecDeque::new();
    d2.resize(3, 7);
    d2.push_front(1);
    println!("{:?} {:?}", d2, d2.iter().position(|&x| x == 7));
    let e: VecDeque<i32> = d2.split_off(2);
    println!("{:?} {:?} eq={}", d2, e, e == VecDeque::from(vec![7, 7]));
}
