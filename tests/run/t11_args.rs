// std::env::args with fixed arguments from t11_args.args (args[0] is a path; never printed)
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("argc {}", args.len());
    println!("has argv0 {}", !args[0].is_empty());
    for (i, a) in args.iter().enumerate().skip(1) {
        println!("arg {} = [{}] len {}", i, a, a.len());
    }
    let mut it = env::args().skip(1);
    let first = it.next().unwrap_or_default();
    println!("first {}", first);
    let rest: Vec<String> = it.collect();
    println!("rest {:?}", rest);
    let mut sum: i64 = 0;
    let mut flags = Vec::new();
    for a in env::args().skip(1) {
        if let Some(f) = a.strip_prefix("--") {
            flags.push(f.to_string());
        } else if let Ok(n) = a.parse::<i64>() {
            sum += n;
        }
    }
    println!("numeric sum {}", sum);
    println!("flags {:?}", flags);
    let os: Vec<std::ffi::OsString> = env::args_os().skip(1).collect();
    println!("args_os count {}", os.len());
    for o in &os {
        println!("os {:?}", o.to_str());
    }
}
