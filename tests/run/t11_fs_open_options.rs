// File::create / File::open / OpenOptions (append, create_new, truncate) / Seek / Read / Write
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

fn main() {
    let dir = "t11_fs_open_tmp";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir(dir).unwrap();
    let path = format!("{}/log.txt", dir);
    {
        let mut f = File::create(&path).unwrap();
        writeln!(f, "first {}", 1).unwrap();
        f.write_all(b"second\n").unwrap();
        let n = f.write(b"third\n").unwrap();
        println!("wrote {} bytes", n);
    }
    {
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, "appended").unwrap();
    }
    let mut s = String::new();
    File::open(&path).unwrap().read_to_string(&mut s).unwrap();
    print!("{}", s);
    println!("create_new on existing is_err {}", OpenOptions::new().write(true).create_new(true).open(&path).is_err());
    {
        let mut f = OpenOptions::new().read(true).write(true).open(&path).unwrap();
        f.seek(SeekFrom::Start(6)).unwrap();
        f.write_all(b"ONE").unwrap();
        let pos = f.seek(SeekFrom::Current(0)).unwrap();
        println!("pos after patch {}", pos);
        let end = f.seek(SeekFrom::End(0)).unwrap();
        println!("end {}", end);
        f.seek(SeekFrom::End(-9)).unwrap();
        let mut tail = String::new();
        f.read_to_string(&mut tail).unwrap();
        println!("tail {:?}", tail);
        f.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = [0u8; 5];
        f.read_exact(&mut buf).unwrap();
        println!("head {:?}", std::str::from_utf8(&buf).unwrap());
    }
    println!("patched {:?}", fs::read_to_string(&path).unwrap());
    {
        let mut f = OpenOptions::new().write(true).truncate(true).open(&path).unwrap();
        f.write_all(b"truncated").unwrap();
    }
    println!("after truncate {:?}", fs::read_to_string(&path).unwrap());
    {
        let f = OpenOptions::new().write(true).open(&path).unwrap();
        f.set_len(4).unwrap();
    }
    println!("after set_len {:?}", fs::read_to_string(&path).unwrap());
    let mut f = File::open(&path).unwrap();
    let mut chunk = [0u8; 3];
    loop {
        let n = f.read(&mut chunk).unwrap();
        if n == 0 {
            break;
        }
        println!("chunk {} {:?}", n, &chunk[..n]);
    }
    println!("open missing kind {:?}", File::open(format!("{}/nope", dir)).unwrap_err().kind());
    fs::remove_dir_all(dir).unwrap();
    println!("done");
}
