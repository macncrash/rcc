// Index and IndexMut with different index types.
use std::ops::{Index, IndexMut};

struct Grid {
    w: usize,
    h: usize,
    cells: Vec<i32>,
}

impl Grid {
    fn new(w: usize, h: usize) -> Grid {
        Grid { w, h, cells: vec![0; w * h] }
    }
}

impl Index<(usize, usize)> for Grid {
    type Output = i32;
    fn index(&self, (r, c): (usize, usize)) -> &i32 {
        &self.cells[r * self.w + c]
    }
}
impl IndexMut<(usize, usize)> for Grid {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut i32 {
        &mut self.cells[r * self.w + c]
    }
}

#[derive(Clone, Copy)]
enum Reg {
    A,
    B,
    C,
}

struct Regs {
    vals: [u64; 3],
}

impl Index<Reg> for Regs {
    type Output = u64;
    fn index(&self, r: Reg) -> &u64 {
        &self.vals[r as usize]
    }
}
impl IndexMut<Reg> for Regs {
    fn index_mut(&mut self, r: Reg) -> &mut u64 {
        &mut self.vals[r as usize]
    }
}

struct Names {
    list: [&'static str; 3],
}
impl Index<&str> for Names {
    type Output = usize;
    fn index(&self, key: &str) -> &usize {
        const IDX: [usize; 3] = [0, 1, 2];
        let mut i = 0;
        while i < 3 {
            if self.list[i] == key {
                return &IDX[i];
            }
            i += 1;
        }
        panic!("no such name");
    }
}

fn main() {
    let mut g = Grid::new(4, 3);
    for r in 0..g.h {
        for c in 0..g.w {
            g[(r, c)] = (r * 10 + c) as i32;
        }
    }
    g[(1, 1)] += 100;
    g[(2, 3)] *= -1;
    for r in 0..g.h {
        for c in 0..g.w {
            print!("{} ", g[(r, c)]);
        }
        println!();
    }
    let mut regs = Regs { vals: [0; 3] };
    regs[Reg::A] = 5;
    regs[Reg::B] = regs[Reg::A] * 7;
    regs[Reg::C] += regs[Reg::B] + 1;
    let rr = &mut regs[Reg::A];
    *rr <<= 3;
    println!("{} {} {}", regs[Reg::A], regs[Reg::B], regs[Reg::C]);
    let n = Names { list: ["x", "y", "z"] };
    println!("{} {} {}", n["z"], n["x"], n["y"]);
}
