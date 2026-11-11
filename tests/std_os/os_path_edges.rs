// Path/PathBuf edge cases: trailing slashes, `.`/`..`, dotfiles, no extension, root, empty, ordering, hashing
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};

fn show(s: &str) {
    let p = Path::new(s);
    let comps: Vec<String> = p
        .components()
        .map(|c| match c {
            Component::RootDir => "/".to_string(),
            Component::CurDir => ".".to_string(),
            Component::ParentDir => "..".to_string(),
            Component::Normal(n) => format!("N({})", n.to_string_lossy()),
            Component::Prefix(_) => "prefix".to_string(),
        })
        .collect();
    let back: Vec<String> = p.components().rev().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    println!(
        "{:?}: name {:?} stem {:?} ext {:?} prefix {:?} parent {:?} abs {} comps {:?} rev {:?} iter {:?}",
        s,
        p.file_name(),
        p.file_stem(),
        p.extension(),
        p.file_name().map(|_| p.file_stem()),
        p.parent(),
        p.is_absolute(),
        comps,
        back,
        p.iter().collect::<Vec<&OsStr>>()
    );
}

fn main() {
    for s in [
        "", "/", "//", "///a//b//", ".", "..", "./", "../", "./a", "a/.", "a/./b", "a/..", "/..", "/.", "foo/", "foo//",
        ".bashrc", "dir/.bashrc", ".bashrc.bak", "file.", "a.b.c", "..x", "x..", "foo..bar", "/etc/hosts", "noext",
        "a/b/../c/./d/", "./../a", "tests/std_os/x.rs",
    ] {
        show(s);
    }
    // parent chains
    for s in ["/a/b/c", "a/b/c", "./a", "/", "a"] {
        let anc: Vec<String> = Path::new(s).ancestors().map(|p| p.display().to_string()).collect();
        println!("ancestors {:?} -> {:?}", s, anc);
    }
    // join / push
    println!("{}", Path::new("").join("a").display());
    println!("{}", Path::new("a/").join("b").display());
    println!("{}", Path::new("a").join("").display());
    println!("{}", Path::new("a").join("/b").display());
    println!("{}", Path::new("/").join("etc").display());
    let mut pb = PathBuf::new();
    pb.push("x");
    pb.push("y/");
    pb.push("z");
    println!("pushed {:?}", pb);
    pb.push("");
    println!("push empty {:?}", pb);
    while pb.pop() {
        println!("pop -> {:?}", pb);
    }
    let mut r = PathBuf::from("/");
    println!("pop root {} {:?}", r.pop(), r);
    let mut r2 = PathBuf::from("/usr");
    println!("pop /usr {} {:?}", r2.pop(), r2);
    // set_extension / with_extension / set_file_name
    for s in ["a.txt", ".bashrc", "dir/", "a.tar.gz", "noext", "..", "", "a/b.c/"] {
        let mut p = PathBuf::from(s);
        let ok = p.set_extension("o");
        println!("set_extension({:?}) -> {} {:?} | with_extension(\"\") {:?} | with_file_name {:?}", s, ok, p, Path::new(s).with_extension(""), Path::new(s).with_file_name("n"));
    }
    // starts_with / ends_with / strip_prefix
    let p = Path::new("/usr/lib/libc.so");
    for b in ["/", "/usr", "/usr/", "/usr/li", "usr", "/usr/lib/libc.so", "/usr//lib"] {
        println!("starts_with {:?} {} strip {:?}", b, p.starts_with(b), p.strip_prefix(b).ok());
    }
    for b in ["libc.so", "lib/libc.so", "c.so", "/usr/lib/libc.so", ""] {
        println!("ends_with {:?} {}", b, p.ends_with(b));
    }
    // equality, ordering, hashing
    println!("eq {} {} {} {}", Path::new("a/b") == Path::new("a/b/"), Path::new("a/./b") == Path::new("a/b"), Path::new("./a") == Path::new("a"), Path::new("a/../b") == Path::new("b"));
    let mut v: Vec<PathBuf> = ["b", "a/b", "a.b", "a", "/z", "a/", "A", "a-b", "a/b/c"].iter().map(PathBuf::from).collect();
    v.sort();
    println!("sorted {:?}", v);
    let mut set: HashSet<PathBuf> = HashSet::new();
    set.insert(PathBuf::from("x/y"));
    set.insert(PathBuf::from("x//y/"));
    set.insert(PathBuf::from("x/./y"));
    println!("set len {} contains {}", set.len(), set.contains(Path::new("x/y")));
    let pb = PathBuf::from("q/r");
    let rp: &Path = &pb;
    println!("cross eq {} {} {}", pb == *rp, rp == pb, pb.as_path() == Path::new("q/r"));
    // conversions
    let os: OsString = pb.clone().into_os_string();
    println!("into_os_string {:?} into_string {:?}", os, os.clone().into_string());
    let from_os = PathBuf::from(OsString::from("from/os"));
    println!("{:?} {:?} {:?}", from_os, PathBuf::from(String::from("from/string")), PathBuf::from(Path::new("from/path")));
    println!("to_str {:?} lossy {} display padded [{:>8}] [{:<8}]", Path::new("é/b").to_str(), Path::new("é/b").to_string_lossy(), Path::new("ab").display(), Path::new("cd").display());
    println!("debug escapes {:?}", Path::new("tab\there\"q\""));
    let collected: PathBuf = vec!["a", "b", "/c", "d"].into_iter().collect();
    println!("collected {:?}", collected);
    let o = OsStr::new("abc");
    println!("osstr {:?} len {} eq str {} {} lossy {} owned {:?}", o, o.len(), o == "abc", "abc" == o, o.to_string_lossy(), o.to_os_string());
    let mut s = OsString::from("x");
    s.push("y");
    s.push(OsStr::new("z"));
    println!("osstring push {:?} {}", s, s.len());
    let cow = Path::new("cow").to_string_lossy();
    println!("cow {}", cow);
    let pb2: PathBuf = "with/str".into();
    println!("into {:?} as_os_str {:?} ext of comps {:?}", pb2, pb2.as_os_str(), pb2.components().as_path());
    let mut it = Path::new("/a/b/c").components();
    it.next();
    it.next_back();
    println!("components as_path middle {:?}", it.as_path());
    println!("components debug {:?}", Path::new("/x/./y/..").components());
    println!("has_root {} {}", Path::new("/r").has_root(), Path::new("r").has_root());
}
