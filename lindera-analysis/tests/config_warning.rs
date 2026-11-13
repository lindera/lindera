#![cfg(unix)]

use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn warning_child() {
    if std::env::var_os("LINDERA_WARNING_CHILD").is_none() {
        return;
    }
    println!("ready");
    lindera_analysis::tokenizer::TokenizerBuilder::new().unwrap();
    lindera_analysis::tokenizer::TokenizerBuilder::new().unwrap();
    println!("done");
}

fn child(stderr: UnixStream) -> std::process::Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "warning_child", "--nocapture"])
        .env("LINDERA_WARNING_CHILD", "1")
        .env_remove("LINDERA_CONFIG_FILE")
        .env(
            "LINDERA_CONFIG_PATH",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/empty-config.yaml"
            ),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::from(OwnedFd::from(stderr)))
        .spawn()
        .unwrap()
}

#[test]
fn deprecated_config_warning_survives_a_full_nonblocking_stderr() {
    let (mut reader, mut writer) = UnixStream::pair().unwrap();
    writer.set_nonblocking(true).unwrap();
    loop {
        match writer.write(&[b'x'; 4096]) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => panic!("fill stderr: {e}"),
        }
    }
    let mut child = child(writer);
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    while line.trim() != "ready" {
        line.clear();
        assert_ne!(stdout.read_line(&mut line).unwrap(), 0);
    }
    // Keep stderr full while the child attempts the warning, then let it drain.
    let until = Instant::now() + Duration::from_millis(100);
    while Instant::now() < until && child.try_wait().unwrap().is_none() {
        std::thread::sleep(Duration::from_millis(1));
    }
    reader.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut output = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => output.extend_from_slice(&buffer[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    child.kill().unwrap();
                    child.wait().unwrap();
                    panic!("warning child did not finish after stderr drained");
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) => panic!("read stderr: {e}"),
        }
    }
    let output = String::from_utf8(output).unwrap();
    assert!(child.wait().unwrap().success(), "warning child failed");
    assert_eq!(output.matches("LINDERA_CONFIG_PATH").count(), 1);
}

#[test]
fn deprecated_config_warning_survives_a_closed_stderr() {
    let (reader, writer) = UnixStream::pair().unwrap();
    drop(reader);
    assert!(child(writer).wait().unwrap().success());
}
