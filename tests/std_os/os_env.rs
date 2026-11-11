// env: args/args_os (from .args), set_var/remove_var/var/var_os/vars, VarError, current_dir/set_current_dir, temp_dir, split/join paths
use std::env;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    println!("args {:?} len {}", args, env::args().len());
    let os: Vec<OsString> = env::args_os().skip(1).collect();
    println!("args_os {:?}", os);
    println!("last arg {:?}", env::args().next_back());

    println!("from .env: {:?} {:?}", env::var("OS_ENV_FILE_VAR"), env::var_os("OS_ENV_FILE_VAR"));
    env::set_var("OS_ENV_A", "alpha");
    println!("set: {:?}", env::var("OS_ENV_A"));
    env::set_var("OS_ENV_A", "beta=gamma");
    println!("overwrite: {:?}", env::var("OS_ENV_A"));
    env::set_var("OS_ENV_EMPTY", "");
    println!("empty: {:?}", env::var("OS_ENV_EMPTY"));
    let mut mine: Vec<(String, String)> = env::vars().filter(|(k, _)| k.starts_with("OS_ENV_")).collect();
    mine.sort();
    println!("vars {:?}", mine);
    env::remove_var("OS_ENV_A");
    let e = env::var("OS_ENV_A").unwrap_err();
    println!("removed: {:?} / {} / var_os {:?}", e, e, env::var_os("OS_ENV_A"));
    env::set_var("OS_ENV_BAD", OsStr::from_bytes(b"ab\xffcd"));
    match env::var("OS_ENV_BAD") {
        Ok(v) => println!("unexpected {}", v),
        Err(e) => println!("not unicode: {:?} / {}", e, e),
    }
    println!("bad var_os bytes {:?}", env::var_os("OS_ENV_BAD").map(|v| v.into_vec()));
    println!("empty key {:?} key with = {:?}", env::var(""), env::var("A=B"));

    let start = env::current_dir().unwrap();
    std::fs::create_dir_all("os_env_tmp/inner").unwrap();
    env::set_current_dir("os_env_tmp/inner").unwrap();
    let now = env::current_dir().unwrap();
    println!("cwd ends_with {} relative probe {}", now.ends_with("os_env_tmp/inner"), std::path::Path::new("../inner").exists());
    println!("chdir missing: {:?}", env::set_current_dir("does/not/exist").unwrap_err().kind());
    env::set_current_dir(&start).unwrap();
    std::fs::remove_dir_all("os_env_tmp").unwrap();
    println!("back {}", env::current_dir().unwrap() == start);

    let exe = env::current_exe().unwrap();
    println!("current_exe absolute {} exists {} name {:?}", exe.is_absolute(), exe.exists(), exe.file_name().is_some());
    println!("temp_dir absolute {}", env::temp_dir().is_absolute());
    let parts: Vec<PathBuf> = env::split_paths("/bin:/usr/bin::rel").collect();
    println!("split_paths {:?}", parts);
    println!("join_paths {:?}", env::join_paths(["/a", "/b c"].iter()));
    println!("join_paths err {}", env::join_paths(["/a:b"].iter()).is_err());
    println!("consts {} {} {:?}", env::consts::FAMILY, env::consts::EXE_SUFFIX, env::consts::DLL_PREFIX);
}
