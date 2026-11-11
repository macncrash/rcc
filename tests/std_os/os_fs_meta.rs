// fs metadata, permissions (PermissionsExt), read_dir entries, symlinks, error kinds and messages
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

fn kind_of<T>(r: std::io::Result<T>) -> String {
    match r {
        Ok(_) => "ok".to_string(),
        Err(e) => format!("{:?} ({})", e.kind(), e),
    }
}

fn main() {
    let root = Path::new("os_fs_meta_tmp");
    let _ = fs::remove_dir_all(root);
    fs::create_dir_all(root.join("sub/deeper")).unwrap();
    let f = root.join("script.sh");
    fs::write(&f, "#!/bin/sh\necho hi\n").unwrap();
    fs::set_permissions(&f, fs::Permissions::from_mode(0o755)).unwrap();
    let m = fs::metadata(&f).unwrap();
    println!("mode {:o} perm mode {:o} readonly {}", m.mode() & 0o777, m.permissions().mode() & 0o7777, m.permissions().readonly());
    println!("perm debug {:?}", m.permissions());
    let mut p = m.permissions();
    p.set_mode(0o444);
    fs::set_permissions(&f, p).unwrap();
    let m2 = fs::metadata(&f).unwrap();
    println!("now readonly {} mode {:o}", m2.permissions().readonly(), m2.permissions().mode() & 0o777);
    println!("write to readonly: {}", kind_of(OpenOptions::new().write(true).open(&f)));
    let mut p = m2.permissions();
    p.set_readonly(false);
    fs::set_permissions(&f, p).unwrap();
    println!("readonly cleared {}", !fs::metadata(&f).unwrap().permissions().readonly());
    println!("size {} nlink {} is_file {} ft {:?}", m.size(), m.nlink(), m.is_file(), m.file_type());
    println!("modified after epoch {}", m.modified().unwrap() > std::time::UNIX_EPOCH);
    let dm = fs::metadata(root).unwrap();
    println!("dir ft is_dir {} is_file {} is_symlink {}", dm.file_type().is_dir(), dm.file_type().is_file(), dm.file_type().is_symlink());

    // symlinks
    std::os::unix::fs::symlink("script.sh", root.join("link")).unwrap();
    let lm = fs::symlink_metadata(root.join("link")).unwrap();
    println!("link is_symlink {} follows to file {}", lm.file_type().is_symlink(), fs::metadata(root.join("link")).unwrap().is_file());
    println!("read_link {:?}", fs::read_link(root.join("link")).unwrap());
    println!("link is_symlink via Path {}", root.join("link").is_symlink());
    std::os::unix::fs::symlink("nowhere", root.join("dangling")).unwrap();
    println!("dangling exists {} try_exists {:?} symlink_metadata ok {}", root.join("dangling").exists(), root.join("dangling").try_exists(), fs::symlink_metadata(root.join("dangling")).is_ok());
    fs::hard_link(&f, root.join("hard")).unwrap();
    println!("hard nlink {}", fs::metadata(&f).unwrap().nlink());

    // read_dir
    File::create(root.join("sub/a.txt")).unwrap().write_all(b"aaa").unwrap();
    File::create(root.join("sub/.hidden")).unwrap();
    let mut entries: Vec<(String, bool, bool, u64)> = fs::read_dir(root)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            let ft = e.file_type().unwrap();
            let len = e.metadata().unwrap().len();
            (e.file_name().to_string_lossy().into_owned(), ft.is_dir(), ft.is_symlink(), if ft.is_dir() { 0 } else { len })
        })
        .collect();
    entries.sort();
    for e in &entries {
        println!("entry {:?}", e);
    }
    let mut subs: Vec<PathBuf> = root.join("sub").read_dir().unwrap().map(|e| e.unwrap().path()).collect();
    subs.sort();
    println!("sub paths {:?}", subs);
    let flat: Vec<String> = fs::read_dir(root.join("sub/deeper")).unwrap().flatten().map(|e| format!("{:?}", e.file_name())).collect();
    println!("empty dir entries {:?}", flat);

    // canonicalize
    let canon = fs::canonicalize(root.join("sub/../script.sh")).unwrap();
    let expect = std::env::current_dir().unwrap().canonicalize().unwrap().join("os_fs_meta_tmp/script.sh");
    println!("canonicalize ok {} absolute {}", canon == expect, canon.is_absolute());
    println!("canonicalize missing {}", kind_of(fs::canonicalize(root.join("missing"))));

    // error paths
    println!("open missing: {}", kind_of(File::open(root.join("missing.txt"))));
    println!("create in missing dir: {}", kind_of(File::create(root.join("nodir/x"))));
    println!("create_dir existing: {}", kind_of(fs::create_dir(root.join("sub"))));
    println!("create_dir missing parent: {}", kind_of(fs::create_dir(root.join("x/y"))));
    println!("remove_dir non-empty: {}", kind_of(fs::remove_dir(root.join("sub"))));
    println!("remove_file on dir: {:?}", fs::remove_file(root.join("sub")).map_err(|e| e.raw_os_error().is_some()));
    println!("open dir for write: {}", kind_of(OpenOptions::new().write(true).open(root.join("sub"))));
    println!("file as dir: {}", kind_of(File::open(root.join("script.sh/x"))));
    println!("read_dir on file: {}", kind_of(fs::read_dir(&f)));
    println!("create_new existing: {}", kind_of(File::create_new(&f)));
    println!("no access mode: {}", kind_of(OpenOptions::new().open(&f)));
    println!("truncate without write: {}", kind_of(OpenOptions::new().read(true).truncate(true).open(&f)));
    println!("rename missing: {}", kind_of(fs::rename(root.join("nope"), root.join("nope2"))));
    println!("copy dir: {}", kind_of(fs::copy(root.join("sub"), root.join("subcopy"))));
    println!("metadata missing is NotFound {}", fs::metadata(root.join("zz")).unwrap_err().kind() == ErrorKind::NotFound);
    println!("exists missing {:?}", fs::exists(root.join("zz")));
    println!("nul in path: {}", kind_of(File::open("a\0b")));

    // remove_dir_all does not follow symlinks
    fs::create_dir(root.join("victim")).unwrap();
    fs::write(root.join("victim/keep"), "k").unwrap();
    std::os::unix::fs::symlink("../victim", root.join("sub/to_victim")).unwrap();
    fs::remove_dir_all(root.join("sub")).unwrap();
    println!("victim kept {}", root.join("victim/keep").exists());
    fs::remove_dir_all(root).unwrap();
    println!("root gone {}", !root.exists());
}
