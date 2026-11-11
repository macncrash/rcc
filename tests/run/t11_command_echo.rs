// std::process::Command running /bin/echo, capturing output and status
use std::process::{Command, Stdio};

fn main() {
    let out = Command::new("/bin/echo").arg("hello").arg("from echo").output().unwrap();
    println!("status success {} code {:?}", out.status.success(), out.status.code());
    println!("stdout {:?}", String::from_utf8(out.stdout.clone()).unwrap());
    println!("stdout bytes {}", out.stdout.len());
    println!("stderr empty {}", out.stderr.is_empty());
    let args = ["a", "b c", "d"];
    let out2 = Command::new("/bin/echo").args(&args).output().unwrap();
    println!("args echo {:?}", String::from_utf8_lossy(&out2.stdout));
    let out3 = Command::new("/bin/echo").arg("-n").arg("no newline").output().unwrap();
    println!("echo -n {:?}", String::from_utf8_lossy(&out3.stdout));
    // status() inherits stdout: the child's output appears in our stdout
    println!("before child");
    use std::io::Write;
    std::io::stdout().flush().unwrap();
    let st = Command::new("/bin/echo").arg("child writes this line").status().unwrap();
    println!("after child, success {}", st.success());
    // spawn + wait_with_output with piped stdout
    let child = Command::new("/bin/echo").arg("piped").stdout(Stdio::piped()).spawn().unwrap();
    let o = child.wait_with_output().unwrap();
    println!("spawned output {:?} code {:?}", String::from_utf8_lossy(&o.stdout), o.status.code());
    // many invocations
    let mut collected = Vec::new();
    for i in 0..5 {
        let o = Command::new("/bin/echo").arg(format!("n={}", i * 11)).output().unwrap();
        collected.push(String::from_utf8(o.stdout).unwrap().trim_end().to_string());
    }
    println!("collected {:?}", collected);
    // missing program
    match Command::new("/nonexistent/program/xyz").output() {
        Ok(_) => println!("unexpected success"),
        Err(e) => println!("spawn error kind {:?}", e.kind()),
    }
}
