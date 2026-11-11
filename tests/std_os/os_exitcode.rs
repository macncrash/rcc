// main returning ExitCode
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    println!("argc {}", args.len());
    if args.len() < 5 { ExitCode::from(7) } else { ExitCode::SUCCESS }
}
