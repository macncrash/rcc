// std::fs write/read/read_to_string/metadata/remove_file/exists in a relative temp dir
use std::fs;
use std::path::Path;

fn main() {
    let dir = "t11_fs_basic_tmp";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    let p = format!("{}/hello.txt", dir);
    fs::write(&p, "line one\nline two\n").unwrap();
    let s = fs::read_to_string(&p).unwrap();
    println!("read_to_string {:?}", s);
    println!("lines {}", s.lines().count());
    let bytes = fs::read(&p).unwrap();
    println!("read {} bytes, first {} last {}", bytes.len(), bytes[0], bytes[bytes.len() - 1]);
    let md = fs::metadata(&p).unwrap();
    println!("metadata len {} is_file {} is_dir {}", md.len(), md.is_file(), md.is_dir());
    let dmd = fs::metadata(dir).unwrap();
    println!("dir is_dir {} is_file {}", dmd.is_dir(), dmd.is_file());
    let bin = format!("{}/data.bin", dir);
    let data: Vec<u8> = (0..=255u8).collect();
    fs::write(&bin, &data).unwrap();
    let back = fs::read(&bin).unwrap();
    println!("binary roundtrip {} len {}", back == data, back.len());
    let sum: u32 = back.iter().map(|&b| b as u32).sum();
    println!("binary sum {}", sum);
    // overwrite truncates
    fs::write(&p, b"short").unwrap();
    println!("after overwrite {:?} len {}", fs::read_to_string(&p).unwrap(), fs::metadata(&p).unwrap().len());
    // copy and rename
    let c = format!("{}/copy.txt", dir);
    let n = fs::copy(&p, &c).unwrap();
    println!("copied {} bytes", n);
    let r = format!("{}/renamed.txt", dir);
    fs::rename(&c, &r).unwrap();
    println!("copy exists {} renamed exists {}", Path::new(&c).exists(), Path::new(&r).exists());
    println!("renamed content {}", fs::read_to_string(&r).unwrap());
    fs::remove_file(&r).unwrap();
    println!("after remove exists {}", Path::new(&r).exists());
    match fs::read_to_string(format!("{}/missing.txt", dir)) {
        Ok(_) => println!("unexpected"),
        Err(e) => println!("missing kind {:?}", e.kind()),
    }
    println!("remove missing is_err {}", fs::remove_file(&r).is_err());
    // invalid utf-8 -> read_to_string fails, read works
    fs::write(&bin, [0xffu8, 0xfe, 0x41]).unwrap();
    println!("bad utf8 read_to_string kind {:?}", fs::read_to_string(&bin).unwrap_err().kind());
    fs::remove_dir_all(dir).unwrap();
    println!("cleaned {}", !Path::new(dir).exists());
}
