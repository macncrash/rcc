// Command exit statuses, env, current_dir, stdin piping, stderr capture via /bin/sh
use std::io::Write;
use std::process::{Command, Stdio};

fn main() {
    for code in [0, 1, 3, 42, 255] {
        let st = Command::new("/bin/sh").arg("-c").arg(format!("exit {}", code)).status().unwrap();
        println!("exit {} -> success {} code {:?}", code, st.success(), st.code());
    }
    let o = Command::new("/bin/sh").arg("-c").arg("echo out; echo err 1>&2; exit 3").output().unwrap();
    println!("stdout {:?} stderr {:?} code {:?}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr), o.status.code());
    let o = Command::new("/bin/sh").arg("-c").arg("echo \"$RCC_CHILD_VAR\"").env("RCC_CHILD_VAR", "set-by-parent").output().unwrap();
    println!("env {:?}", String::from_utf8_lossy(&o.stdout));
    let o = Command::new("/bin/sh").arg("-c").arg("echo \"[$RCC_CHILD_VAR]\"").env_remove("RCC_CHILD_VAR").output().unwrap();
    println!("env_remove {:?}", String::from_utf8_lossy(&o.stdout));
    std::fs::create_dir_all("t11_cmd_dir/sub").unwrap();
    let o = Command::new("/bin/sh").arg("-c").arg("basename \"$(pwd)\"").current_dir("t11_cmd_dir/sub").output().unwrap();
    println!("current_dir {:?}", String::from_utf8_lossy(&o.stdout));
    std::fs::remove_dir_all("t11_cmd_dir").unwrap();
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg("tr a-z A-Z")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(b"shout this\nand this\n").unwrap();
    }
    let o = child.wait_with_output().unwrap();
    println!("tr {:?} success {}", String::from_utf8_lossy(&o.stdout), o.status.success());
    let mut c = Command::new("/bin/sh").arg("-c").arg("exit 7").spawn().unwrap();
    let st = c.wait().unwrap();
    println!("wait code {:?}", st.code());
    let o = Command::new("/bin/sh").arg("-c").arg("printf 'a\\0b'").stdout(Stdio::piped()).stderr(Stdio::null()).output().unwrap();
    println!("binary stdout {:?}", o.stdout);
}
