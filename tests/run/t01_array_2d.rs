const N: usize = 4;

fn transpose(m: [[i32; N]; N]) -> [[i32; N]; N] {
    let mut t = [[0; N]; N];
    for i in 0..N {
        for j in 0..N {
            t[j][i] = m[i][j];
        }
    }
    t
}

fn print_m(m: &[[i32; N]; N]) {
    for i in 0..N {
        for j in 0..N {
            print!("{} ", m[i][j]);
        }
        println!();
    }
}

fn main() {
    let mut m = [[0i32; N]; N];
    for i in 0..N {
        for j in 0..N {
            m[i][j] = (i * N + j) as i32;
        }
    }
    print_m(&m);
    let t = transpose(m);
    print_m(&t);
    let mut diag = 0;
    for i in 0..N {
        diag += m[i][i];
    }
    println!("trace = {}", diag);
    let mut grid = [[' '; 5]; 3];
    grid[1][2] = '#';
    grid[0][0] = '@';
    grid[2][4] = '*';
    for row in grid {
        print!("|");
        for c in row {
            print!("{}", c);
        }
        println!("|");
    }
    let cube = [[[1u8; 2]; 3]; 4];
    let mut total = 0u32;
    for a in cube {
        for b in a {
            for c in b {
                total += c as u32;
            }
        }
    }
    println!("cube total = {}", total);
    let row = m[2];
    println!("row2 = {} {} {} {}", row[0], row[1], row[2], row[3]);
    m[3] = [9, 9, 9, 9];
    print_m(&m);
}
