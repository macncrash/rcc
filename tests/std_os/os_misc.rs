// BufReader seek, File try_clone/sync/&File io, OpenOptionsExt::mode, DirBuilder, CString, ? into Box<dyn Error>
use std::error::Error;
use std::ffi::CString;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

fn run() -> Result<u32, Box<dyn Error>> {
    let dir = Path::new("os_misc_tmp");
    let _ = fs::remove_dir_all(dir);
    DirBuilder::new().recursive(true).mode(0o750).create(dir.join("a/b"))?;
    println!("dir mode {:o}", fs::metadata(dir.join("a/b"))?.permissions().mode() & 0o777);
    let p = dir.join("data.txt");
    let mut f = OpenOptions::new().write(true).create(true).mode(0o600).open(&p)?;
    for i in 0..10 {
        writeln!(f, "row {}", i)?;
    }
    f.sync_all()?;
    println!("file mode {:o}", f.metadata()?.permissions().mode() & 0o777);
    let mut g = f.try_clone()?;
    g.write_all(b"via clone\n")?;
    (&f).write_all(b"via &File\n")?;
    drop(f);
    drop(g);
    let mut r = BufReader::with_capacity(16, File::open(&p)?);
    let mut line = String::new();
    r.read_line(&mut line)?;
    println!("first {:?} stream_position {}", line, r.stream_position()?);
    r.seek(SeekFrom::Current(6))?;
    line.clear();
    r.read_line(&mut line)?;
    println!("after seek {:?}", line);
    r.seek(SeekFrom::End(-10))?;
    line.clear();
    r.read_line(&mut line)?;
    println!("last {:?}", line);
    let mut whole = String::new();
    File::open(&p)?.read_to_string(&mut whole)?;
    println!("lines {}", whole.lines().count());
    let n: u32 = "17".parse()?;
    let c = CString::new("hi there")?;
    println!("cstring {:?} bytes {} with nul {}", c, c.as_bytes().len(), c.as_bytes_with_nul().len());
    println!("nul error {}", CString::new(vec![b'a', 0, b'b']).unwrap_err());
    fs::remove_dir_all(dir)?;
    let _missing = fs::read("os_misc_missing")?;
    Ok(n)
}

fn main() {
    match run() {
        Ok(n) => println!("ok {}", n),
        Err(e) => println!("error: {}", e),
    }
    let r: Result<u8, Box<dyn Error>> = "300".parse::<u8>().map_err(|e| e.into());
    println!("{}", r.unwrap_err());
}
