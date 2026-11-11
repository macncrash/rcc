fn main() {
    'outer: for i in 0..10 {
        for j in 0..10 {
            if i * j == 42 {
                println!("found {} {}", i, j);
                break 'outer;
            }
        }
    }
    'rows: for i in 0..5 {
        for j in 0..5 {
            if j > i {
                continue 'rows;
            }
            if i == 4 {
                break 'rows;
            }
            print!("{}{} ", i, j);
        }
    }
    println!();
    let mut n = 0;
    'a: loop {
        'b: loop {
            n += 1;
            if n % 2 == 0 {
                continue 'a;
            }
            if n > 9 {
                break 'a;
            }
            if n % 3 == 0 {
                break 'b;
            }
            println!("inner n={}", n);
        }
        println!("after b n={}", n);
    }
    println!("final n={}", n);
    let mut i = 0;
    'w: while i < 100 {
        i += 1;
        let mut j = 0;
        while j < i {
            j += 1;
            if j == 7 {
                continue 'w;
            }
            if i * j > 60 {
                break 'w;
            }
        }
        println!("completed i={}", i);
    }
    println!("i={}", i);
    let v = 'search: loop {
        for k in 100..200 {
            if k % 17 == 0 && k % 3 == 0 {
                break 'search k;
            }
        }
        break 0;
    };
    println!("v={}", v);
    let blk = 'blk: {
        if v > 150 {
            break 'blk 1;
        }
        if v > 100 {
            break 'blk 2;
        }
        3
    };
    println!("blk={}", blk);
}
