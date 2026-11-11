// create_dir_all, create_dir, read_dir (sorted), remove_dir, remove_dir_all, nested trees
use std::fs;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, depth: usize, out: &mut Vec<String>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for e in entries {
        let name = e.file_name().unwrap().to_string_lossy().into_owned();
        if e.is_dir() {
            out.push(format!("{}{}/", "  ".repeat(depth), name));
            walk(&e, depth + 1, out);
        } else {
            let len = fs::metadata(&e).unwrap().len();
            out.push(format!("{}{} ({} bytes)", "  ".repeat(depth), name, len));
        }
    }
}

fn main() {
    let root = Path::new("t11_fs_dirs_tmp");
    let _ = fs::remove_dir_all(root);
    fs::create_dir_all(root.join("a/b/c")).unwrap();
    fs::create_dir_all(root.join("a/b/c")).unwrap(); // idempotent
    fs::create_dir(root.join("z")).unwrap();
    println!("create_dir existing is_err {}", fs::create_dir(root.join("z")).is_err());
    for (i, name) in ["m.txt", "b.txt", "a/x.rs", "a/b/y.rs", "a/b/c/deep.txt", "z/last"].iter().enumerate() {
        fs::write(root.join(name), "x".repeat(i * 3 + 1)).unwrap();
    }
    let mut out = Vec::new();
    walk(root, 0, &mut out);
    for l in &out {
        println!("{}", l);
    }
    let mut names: Vec<String> = fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    println!("top level {:?}", names);
    let mut kinds: Vec<(String, bool)> = fs::read_dir(root.join("a"))
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (e.file_name().to_string_lossy().into_owned(), e.file_type().unwrap().is_dir())
        })
        .collect();
    kinds.sort();
    println!("a/ entries {:?}", kinds);
    println!("remove non-empty dir is_err {}", fs::remove_dir(root.join("a/b/c")).is_err());
    fs::remove_file(root.join("a/b/c/deep.txt")).unwrap();
    fs::remove_dir(root.join("a/b/c")).unwrap();
    println!("c exists {}", root.join("a/b/c").exists());
    fs::remove_dir_all(root.join("a")).unwrap();
    let mut out2 = Vec::new();
    walk(root, 0, &mut out2);
    println!("after removing a: {:?}", out2);
    fs::remove_dir_all(root).unwrap();
    println!("root exists {}", root.exists());
}
