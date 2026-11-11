// main returning Err(io::Error) exits with status 1 (message on stderr); stdout before is kept
use std::fs::File;
use std::io;

fn main() -> io::Result<()> {
    println!("before");
    print!("no newline");
    let _f = File::open("os_main_result_missing_file")?;
    println!("not reached");
    Ok(())
}
