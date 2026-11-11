// Large file round trips (several MB) through fs::write/read, BufWriter/BufReader, File seek and copy
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};

fn checksum(data: &[u8]) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

fn main() {
    let dir = "os_fs_large_tmp";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir(dir).unwrap();
    let big: Vec<u8> = (0..6_000_000u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect();
    let p = format!("{}/big.bin", dir);
    fs::write(&p, &big).unwrap();
    println!("metadata len {}", fs::metadata(&p).unwrap().len());
    let back = fs::read(&p).unwrap();
    println!("read back len {} equal {} checksum {:x}", back.len(), back == big, checksum(&back));

    // BufWriter text, line by line
    let tp = format!("{}/lines.txt", dir);
    {
        let mut w = BufWriter::new(File::create(&tp).unwrap());
        for i in 0..200_000 {
            writeln!(w, "{} {}", i, i * 3 % 1009).unwrap();
        }
    }
    let r = BufReader::new(File::open(&tp).unwrap());
    let mut count = 0u64;
    let mut sum = 0u64;
    for line in r.lines() {
        let line = line.unwrap();
        let mut it = line.split(' ');
        let a: u64 = it.next().unwrap().parse().unwrap();
        let b: u64 = it.next().unwrap().parse().unwrap();
        count += 1;
        sum += a + b;
    }
    println!("lines {} sum {} size {}", count, sum, fs::metadata(&tp).unwrap().len());
    let s = fs::read_to_string(&tp).unwrap();
    println!("read_to_string len {} last line {:?}", s.len(), s.lines().last());

    // copy and compare
    let cp = format!("{}/copy.bin", dir);
    let n = fs::copy(&p, &cp).unwrap();
    println!("copy {} equal {}", n, fs::read(&cp).unwrap() == big);

    // random access with seek
    let mut f = OpenOptions::new().read(true).write(true).open(&cp).unwrap();
    let mut buf = [0u8; 8];
    f.seek(SeekFrom::Start(4_000_000)).unwrap();
    f.read_exact(&mut buf).unwrap();
    println!("at 4M {:?} expected {:?}", buf, &big[4_000_000..4_000_008]);
    f.seek(SeekFrom::End(-3)).unwrap();
    f.write_all(b"END").unwrap();
    let end = f.seek(SeekFrom::Current(0)).unwrap();
    println!("end pos {}", end);
    f.set_len(5_000_000).unwrap();
    drop(f);
    let m = fs::metadata(&cp).unwrap();
    println!("truncated len {}", m.len());
    let mut f = File::open(&cp).unwrap();
    let mut all = Vec::new();
    let n = f.read_to_end(&mut all).unwrap();
    println!("read_to_end {} equal prefix {}", n, all[..] == big[..5_000_000]);

    // append in many small writes
    let ap = format!("{}/append.log", dir);
    for i in 0..50 {
        let mut f = OpenOptions::new().create(true).append(true).open(&ap).unwrap();
        write!(f, "{};", i).unwrap();
    }
    let a = fs::read_to_string(&ap).unwrap();
    println!("append len {} tail {:?}", a.len(), &a[a.len() - 9..]);

    // stdin-like read in fixed chunks
    let mut f = File::open(&p).unwrap();
    let mut chunk = vec![0u8; 65536];
    let mut total = 0usize;
    let mut chunks = 0;
    loop {
        let n = f.read(&mut chunk).unwrap();
        if n == 0 {
            break;
        }
        total += n;
        chunks += 1;
    }
    println!("chunked total {} chunks>=92 {}", total, chunks >= 92);
    fs::remove_dir_all(dir).unwrap();
    println!("cleaned {}", !std::path::Path::new(dir).exists());
}
