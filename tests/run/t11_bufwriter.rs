// BufWriter over a File and over stdout; data visible only after flush/drop
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};

fn main() {
    let dir = "t11_bufwriter_tmp";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir(dir).unwrap();
    let path = format!("{}/out.txt", dir);
    {
        let f = File::create(&path).unwrap();
        let mut w = BufWriter::new(f);
        for i in 0..100 {
            writeln!(w, "line {} value {}", i, i * 37 % 101).unwrap();
        }
        println!("buffered size before flush {}", fs::metadata(&path).unwrap().len());
        w.flush().unwrap();
        println!("size after flush {}", fs::metadata(&path).unwrap().len());
        write!(w, "tail").unwrap();
        // dropped here: BufWriter flushes on drop
    }
    let s = fs::read_to_string(&path).unwrap();
    println!("final size {} lines {} ends with tail {}", s.len(), s.lines().count(), s.ends_with("tail"));
    println!("line 42: {}", s.lines().nth(42).unwrap());
    {
        let f = File::create(&path).unwrap();
        let mut w = BufWriter::with_capacity(16, f);
        w.write_all(b"0123456789").unwrap();
        println!("cap {} buffered {}", w.capacity(), w.buffer().len());
        w.write_all(b"abcdefghij").unwrap();
        let f = w.into_inner().unwrap();
        println!("into_inner len {}", f.metadata().unwrap().len());
    }
    println!("contents {:?}", fs::read_to_string(&path).unwrap());
    {
        let stdout = io::stdout();
        let mut out = BufWriter::new(stdout.lock());
        for i in 0..10 {
            write!(out, "{},", i).unwrap();
        }
        writeln!(out).unwrap();
        out.flush().unwrap();
    }
    // BufWriter over a Vec<u8>
    let mut v = BufWriter::new(Vec::new());
    write!(v, "into vec {}", 7).unwrap();
    let bytes = v.into_inner().unwrap();
    println!("{}", String::from_utf8(bytes).unwrap());
    fs::remove_dir_all(dir).unwrap();
}
