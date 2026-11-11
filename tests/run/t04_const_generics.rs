// Const generics: arrays of any length, const params on structs.
fn sum<const N: usize>(a: [i32; N]) -> i32 {
    let mut s = 0;
    for x in a {
        s += x;
    }
    s
}

fn first_last<T: Copy, const N: usize>(a: &[T; N]) -> (T, T) {
    (a[0], a[N - 1])
}

#[derive(Debug, Clone, Copy)]
struct Buf<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> Buf<N> {
    fn new() -> Self {
        Buf { data: [0; N], len: 0 }
    }
    fn push(&mut self, b: u8) -> bool {
        if self.len == N {
            return false;
        }
        self.data[self.len] = b;
        self.len += 1;
        true
    }
    fn capacity(&self) -> usize {
        N
    }
    fn contents(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

struct Matrix<const R: usize, const C: usize> {
    m: [[i32; C]; R],
}

impl<const R: usize, const C: usize> Matrix<R, C> {
    fn transpose(&self) -> Matrix<C, R> {
        let mut m = [[0; R]; C];
        for i in 0..R {
            for j in 0..C {
                m[j][i] = self.m[i][j];
            }
        }
        Matrix { m }
    }
    fn print(&self) {
        for row in &self.m {
            for v in row {
                print!("{} ", v);
            }
            println!();
        }
    }
}

fn mul<const A: usize, const B: usize, const C: usize>(x: &Matrix<A, B>, y: &Matrix<B, C>) -> Matrix<A, C> {
    let mut m = [[0; C]; A];
    for i in 0..A {
        for j in 0..C {
            for k in 0..B {
                m[i][j] += x.m[i][k] * y.m[k][j];
            }
        }
    }
    Matrix { m }
}

fn main() {
    println!("{}", sum([1, 2, 3]));
    println!("{}", sum([10; 7]));
    println!("{}", sum([]));
    println!("{:?}", first_last(&['a', 'b', 'c', 'd']));
    println!("{:?}", first_last(&[9u64]));
    let mut b: Buf<3> = Buf::new();
    for i in 0..5 {
        println!("push {} -> {}", i, b.push(i * 11));
    }
    println!("{:?} cap {}", b.contents(), b.capacity());
    let b2 = Buf::<8>::new();
    println!("{} {}", b2.capacity(), b2.contents().len());
    let m = Matrix { m: [[1, 2, 3], [4, 5, 6]] };
    m.print();
    let t = m.transpose();
    t.print();
    let p = mul(&m, &t);
    p.print();
    mul(&t, &m).print();
}
