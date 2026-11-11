// Command: >1 MB on both stdout and stderr, exit codes, signals, kill, try_wait, stdin pipes, env, Debug
use std::io::{Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, ExitStatus, Stdio};

fn main() {
    // 1.5 MB on each stream, interleaved in 50 KB pieces: needs concurrent draining
    let script = "i=0; while [ $i -lt 30 ]; do head -c 50000 /dev/zero | tr '\\0' o; head -c 50000 /dev/zero | tr '\\0' e >&2; i=$((i+1)); done; exit 5";
    let out = Command::new("/bin/sh").arg("-c").arg(script).output().unwrap();
    println!(
        "big: stdout {} all o {} stderr {} all e {} code {:?} success {}",
        out.stdout.len(),
        out.stdout.iter().all(|&b| b == b'o'),
        out.stderr.len(),
        out.stderr.iter().all(|&b| b == b'e'),
        out.status.code(),
        out.status.success()
    );
    // spawn + read stdout handle directly while stderr is null
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg("head -c 2000000 /dev/zero; echo oops >&2")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut data = Vec::new();
    child.stdout.take().unwrap().read_to_end(&mut data).unwrap();
    let st = child.wait().unwrap();
    println!("direct read {} zeros {} {}", data.len(), data.iter().all(|&b| b == 0), st);

    // exit codes and Display/Debug
    for code in [0, 1, 2, 127, 255] {
        let st = Command::new("/bin/sh").arg("-c").arg(format!("exit {}", code)).status().unwrap();
        println!("exit {}: {} | {:?} | code {:?} signal {:?}", code, st, st, st.code(), st.signal());
    }
    // signal exits
    let st = Command::new("/bin/sh").arg("-c").arg("kill -9 $$").status().unwrap();
    println!("kill -9: {} code {:?} signal {:?} success {}", st, st.code(), st.signal(), st.success());
    let st = Command::new("/bin/sh").arg("-c").arg("kill -TERM $$").status().unwrap();
    println!("kill -TERM: {} signal {:?}", st, st.signal());
    let raw = ExitStatus::from_raw(6 | 0x80);
    println!("from_raw core: {} core_dumped {}", raw, raw.core_dumped());
    // kill a long-running child
    let mut sleeper = Command::new("/bin/sleep").arg("30").spawn().unwrap();
    println!("try_wait running {:?}", sleeper.try_wait().unwrap().map(|s| s.code()));
    sleeper.kill().unwrap();
    let st = sleeper.wait().unwrap();
    println!("killed: {} signal {:?}", st, st.signal());
    println!("wait again same {}", sleeper.wait().unwrap() == st);
    println!("kill after exit ok {}", sleeper.kill().is_ok());

    // stdin: 1 MB through wc -c
    let mut wc = Command::new("/usr/bin/wc").arg("-c").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    {
        let mut si = wc.stdin.take().unwrap();
        let block = vec![b'x'; 1 << 20];
        si.write_all(&block).unwrap();
    }
    let o = wc.wait_with_output().unwrap();
    println!("wc -c {:?}", String::from_utf8_lossy(&o.stdout).trim());
    // stdin null by default for output()
    let o = Command::new("/bin/cat").output().unwrap();
    println!("cat with null stdin: {:?} {}", o.stdout, o.status);

    // env handling
    let o = Command::new("/bin/sh").arg("-c").arg("echo \"[$A][$B][$PATH_IS_SET]\"").env("A", "1").env("B", "two words").env("A", "3").output().unwrap();
    println!("env {:?}", String::from_utf8_lossy(&o.stdout));
    let o = Command::new("/usr/bin/env").env_clear().env("ONLY", "me").output().unwrap();
    println!("env_clear {:?}", String::from_utf8_lossy(&o.stdout));
    std::env::set_var("OS_PROCESS_INHERITED", "yes");
    let o = Command::new("/bin/sh").arg("-c").arg("echo $OS_PROCESS_INHERITED").output().unwrap();
    println!("inherited {:?}", String::from_utf8_lossy(&o.stdout));
    let o = Command::new("/bin/sh").arg("-c").arg("echo \"[$OS_PROCESS_INHERITED]\"").env_remove("OS_PROCESS_INHERITED").output().unwrap();
    println!("removed {:?}", String::from_utf8_lossy(&o.stdout));
    // PATH lookup and current_dir
    let o = Command::new("sh").args(["-c", "echo found via PATH"]).output().unwrap();
    println!("path lookup {:?}", String::from_utf8_lossy(&o.stdout));
    let o = Command::new("pwd").current_dir("/").output().unwrap();
    println!("cwd / {:?}", String::from_utf8_lossy(&o.stdout));

    // errors
    let e = Command::new("/nonexistent/prog").spawn().unwrap_err();
    println!("missing: {} | {:?}", e, e.kind());
    let e = Command::new("definitely-not-a-program-xyz").output().unwrap_err();
    println!("missing in PATH: {:?}", e.kind());
    let e = Command::new("/bin/sh").current_dir("/nonexistent/dir").status().unwrap_err();
    println!("bad cwd: {:?}", e.kind());
    let e = Command::new("/etc/hosts").status().unwrap_err();
    println!("not executable: {:?}", e.kind());

    // Debug forms
    let mut c = Command::new("ls");
    c.arg("-l").arg("a b").env("Z", "1").env("A", "x y").current_dir("/tmp");
    println!("{:?}", c);
    let mut c2 = Command::new("prog");
    c2.env_remove("HOME");
    println!("{:?}", c2);
    println!("get_program {:?} args {:?}", c.get_program(), c.get_args().collect::<Vec<_>>());
    let o = Command::new("/bin/echo").arg("dbg").output().unwrap();
    println!("{:?}", o);
    println!("{:?} {:?}", std::process::ExitCode::SUCCESS, std::process::ExitCode::from(9));
    println!("id nonzero {}", std::process::id() > 0);
}
