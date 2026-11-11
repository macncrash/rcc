// Path / PathBuf manipulation on relative paths
use std::path::{Path, PathBuf};

fn describe(p: &Path) {
    println!(
        "{}: file_name {:?} stem {:?} ext {:?} parent {:?} rel {} components {}",
        p.display(),
        p.file_name(),
        p.file_stem(),
        p.extension(),
        p.parent(),
        p.is_relative(),
        p.components().count()
    );
}

fn main() {
    for s in ["src/main.rs", "a/b/c.tar.gz", "noext", ".hidden", "dir/", "x/../y.txt", "/abs/path.o", "", "a.b/c"] {
        describe(Path::new(s));
    }
    let mut p = PathBuf::from("build");
    p.push("obj");
    p.push("file.c");
    println!("pushed {}", p.display());
    p.set_extension("o");
    println!("set_extension {}", p.display());
    p.set_file_name("other.s");
    println!("set_file_name {}", p.display());
    println!("pop {} -> {}", p.pop(), p.display());
    let j = Path::new("include").join("sys").join("types.h");
    println!("join {}", j.display());
    println!("join absolute replaces {}", Path::new("rel").join("/etc/x").display());
    println!("with_extension {}", j.with_extension("hpp").display());
    println!("with_extension empty {}", Path::new("a/b.c").with_extension("").display());
    println!("with_file_name {}", j.with_file_name("stdio.h").display());
    println!("starts_with {} {}", j.starts_with("include"), j.starts_with("inc"));
    println!("ends_with {}", j.ends_with("sys/types.h"));
    println!("strip_prefix {:?}", j.strip_prefix("include").map(|p| p.display().to_string()));
    let comps: Vec<String> = Path::new("a/./b/../c").components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    println!("components {:?}", comps);
    let anc: Vec<String> = Path::new("x/y/z").ancestors().map(|a| a.display().to_string()).collect();
    println!("ancestors {:?}", anc);
    println!("to_str {:?} to_string_lossy {}", Path::new("q.rs").to_str(), Path::new("q.rs").to_string_lossy());
    let mut pb: PathBuf = ["tests", "run", "t11.rs"].iter().collect();
    println!("collected {}", pb.display());
    pb = pb.with_extension("expected");
    println!("expected file {}", pb.display());
    println!("exists (relative, nonexistent) {}", Path::new("definitely/not/here.txt").exists());
    std::fs::write("t11_path_probe.txt", "x").unwrap();
    let probe = Path::new("t11_path_probe.txt");
    println!("probe exists {} is_file {} is_dir {}", probe.exists(), probe.is_file(), probe.is_dir());
    std::fs::remove_file(probe).unwrap();
    println!("probe exists after remove {}", probe.exists());
    println!("eq {}", Path::new("a/b") == Path::new("a//b"));
    println!("as_os_str len {}", Path::new("abc/def").as_os_str().len());
}
