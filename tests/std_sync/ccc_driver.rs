// Replica of ccc's src/lib.rs driver: the real work runs on a thread with a
// 1 GB stack, the handle is joined, a panic payload is downcast to &str /
// String, and the process exits with 1. Also deep recursion (1,000,000
// frames) inside the big thread, and runtime panics (index out of bounds,
// division by zero) caught by join.

fn deep(n: u32, prev: &[u64; 4]) -> u64 {
    let local = [prev[1] ^ n as u64, prev[2].wrapping_add(1), prev[3] ^ prev[0], n as u64];
    if n == 0 {
        return local[0] ^ local[1] ^ local[2];
    }
    let r = deep(n - 1, &local);
    // use `local` after the call so the recursion is not a loop
    r.wrapping_mul(31).wrapping_add(local[(n % 4) as usize])
}

struct Driver {
    mode: u32,
}

impl Driver {
    fn run(&self) -> Result<u64, String> {
        match self.mode {
            0 => Ok(deep(1_000_000, &[1, 2, 3, 4])),
            1 => Err("no input files".to_string()),
            2 => panic!("literal message"),
            3 => panic!("formatted message {} {}", self.mode, "x"),
            4 => {
                let v: Vec<u32> = (0..self.mode).collect();
                let i = v.len() + self.mode as usize;
                Ok(v[i] as u64)
            }
            5 => {
                let z = self.mode - 5;
                Ok((100 / z) as u64)
            }
            6 => {
                let o: Option<u32> = None;
                Ok(o.unwrap() as u64)
            }
            7 => {
                let r: Result<u32, String> = Err("bad".to_string());
                Ok(r.expect("expect failed") as u64)
            }
            _ => unreachable!(),
        }
    }
}

fn describe(e: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = e.downcast_ref::<&str>() {
        format!("&str payload: {}", s)
    } else if let Some(s) = e.downcast_ref::<String>() {
        format!("String payload: {}", s)
    } else {
        "unknown payload".to_string()
    }
}

fn run_mode(mode: u32) -> Option<String> {
    const STACK_SIZE: usize = 1024 * 1024 * 1024; // 1 GB
    let builder = std::thread::Builder::new().stack_size(STACK_SIZE);
    let handler = builder
        .spawn(move || {
            let driver = Driver { mode };
            driver.run()
        })
        .expect("failed to spawn main thread");
    match handler.join() {
        Ok(Ok(v)) => {
            println!("mode {}: ok {}", mode, v);
            None
        }
        Ok(Err(e)) => {
            println!("mode {}: error: {}", mode, e);
            None
        }
        Err(e) => {
            let d = describe(&e);
            println!("mode {}: internal error: {}", mode, d);
            Some(d)
        }
    }
}

fn main() {
    let mut panics = 0;
    for mode in 0..8 {
        if run_mode(mode).is_some() {
            panics += 1;
        }
    }
    println!("panicked threads: {}", panics);
    // the main thread is still fine after all of that
    let v: Vec<u64> = (1..=5).map(|x| x * x).collect();
    println!("main still runs: {:?}", v);

    // exactly ccc's compiler_main tail
    let handler = std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(|| -> Result<(), String> { panic!("internal compiler error at stage {}", 3) })
        .expect("failed to spawn main thread");
    match handler.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("ccc: error: {}", e);
            std::process::exit(1);
        }
        Err(e) => {
            if let Some(s) = e.downcast_ref::<&str>() {
                println!("ccc: internal error: {}", s);
            } else if let Some(s) = e.downcast_ref::<String>() {
                println!("ccc: internal error: {}", s);
            } else {
                println!("ccc: internal error (thread panicked)");
            }
            std::process::exit(1);
        }
    }
}
