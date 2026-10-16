//! A deterministic helper program for the `local-process` provider's real-process tests.
//!
//! It is driven by its first argument and does exactly one thing, so tests exercise the provider
//! with a real child process and without a shell. It is a test tool, not part of the product.

// A helper program talks to the tests through stdout, stderr and the exit code.
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::expect_used)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, ExitCode};
use std::thread::sleep;
use std::time::Duration;

fn arg(args: &[String], index: usize) -> &str {
    args.get(index).map_or("", String::as_str)
}

fn number(args: &[String], index: usize) -> u64 {
    arg(args, index).parse().unwrap_or(0)
}

fn heartbeat(path: &str) -> ! {
    let mut count: u64 = 0;
    loop {
        count += 1;
        let _ = fs::write(path, count.to_string());
        sleep(Duration::from_millis(40));
    }
}

fn wait_for(path: &str) {
    for _ in 0..200 {
        if Path::new(path).exists() {
            return;
        }
        sleep(Duration::from_millis(25));
    }
}

// The grandchild is meant to outlive this process; the provider must clean it up.
#[allow(clippy::zombie_processes)]
fn spawn_heartbeat(path: &str) {
    let exe = std::env::current_exe().expect("current executable");
    Command::new(exe)
        .args(["heartbeat", path])
        .spawn()
        .expect("spawn the heartbeat child");
    wait_for(path);
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut stdout = io::stdout();
    match arg(&args, 0) {
        "echo-input" => {
            let input = fs::read(arg(&args, 1)).expect("read the input");
            let mut out = b"out:".to_vec();
            out.extend_from_slice(&input);
            fs::write(arg(&args, 2), out).expect("write the output");
            let _ = stdout.write_all(b"ok\n");
        }
        "exit" => return ExitCode::from(u8::try_from(number(&args, 1)).unwrap_or(1)),
        "sleep" => {
            // Lets tests measure the timeout from when the child really started, however long
            // the operating system took to launch it.
            let _ = fs::write("started", b"");
            sleep(Duration::from_millis(number(&args, 1)));
        }
        "flood" => {
            let chunk = vec![b'x'; 8192];
            let mut left = number(&args, 1);
            let mut stderr = io::stderr();
            while left > 0 {
                let n = usize::try_from(left.min(8192)).unwrap_or(8192);
                let _ = stdout.write_all(&chunk[..n]);
                if arg(&args, 2) == "stderr" {
                    let _ = stderr.write_all(&chunk[..n]);
                }
                left -= n as u64;
            }
        }
        "binary" => {
            let _ = stdout.write_all(&[0, 255, 254, 128, 10, 13, 0]);
        }
        "env" => {
            let vars: BTreeMap<String, String> = std::env::vars().collect();
            for (name, value) in vars {
                let _ = writeln!(stdout, "{name}={value}");
            }
        }
        "cwd" => {
            let cwd = std::env::current_dir().expect("current dir");
            let _ = write!(stdout, "{}", cwd.display());
        }
        "write" => fs::write(arg(&args, 1), arg(&args, 2)).expect("write the file"),
        "big-output" => {
            let size = usize::try_from(number(&args, 2)).unwrap_or(0);
            fs::write(arg(&args, 1), vec![b'x'; size]).expect("write the big file");
        }
        "dir-output" => fs::create_dir_all(arg(&args, 1)).expect("create the directory"),
        // `big-outputs <count> <size> <last-size>` writes outputs/o0 .. outputs/o<count-1>.
        "big-outputs" => {
            let count = number(&args, 1);
            for n in 0..count {
                let size = if n + 1 == count {
                    number(&args, 3)
                } else {
                    number(&args, 2)
                };
                let size = usize::try_from(size).unwrap_or(0);
                fs::write(format!("outputs/o{n}"), vec![b'x'; size]).expect("write an output");
            }
        }
        "stdin-eof" => {
            let mut buffer = Vec::new();
            let _ = io::stdin().read_to_end(&mut buffer);
            let _ = write!(stdout, "{}", buffer.len());
        }
        "stderr-exit" => {
            let _ = io::stderr().write_all(b"boom\n");
            return ExitCode::from(u8::try_from(number(&args, 1)).unwrap_or(1));
        }
        "heartbeat" => heartbeat(arg(&args, 1)),
        // A grandchild keeps running while this process waits: cancellation must reach it.
        "spawn-child" => {
            spawn_heartbeat(arg(&args, 1));
            sleep(Duration::from_millis(number(&args, 2)));
        }
        // The grandchild outlives this process: the provider must not leave it behind.
        "spawn-and-exit" => spawn_heartbeat(arg(&args, 1)),
        other => {
            let _ = writeln!(io::stderr(), "unknown fixture command `{other}`");
            return ExitCode::from(64);
        }
    }
    ExitCode::SUCCESS
}
