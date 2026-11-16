//! The bytes that `console::write` and `console::write_line` put on a
//! standard stream.
//!
//! The test runs this test binary again as a child process that writes to
//! its standard error, so that the bytes on the real stream can be compared.

#![cfg(feature = "console")]

use std::env;
use std::process::{Command, Stdio};

use lindera_crf::console::{Stream, write, write_line};

/// Set in the environment of the child process, which then writes.
const CHILD_ENV: &str = "LINDERA_CRF_TEST_CONSOLE_CHILD";

/// Writes a progress text and a line to stderr when run as the child
/// process of the test below, and does nothing otherwise.
#[test]
#[ignore = "run as a child process by the other test in this file"]
fn child_writes_to_stderr() {
    if env::var_os(CHILD_ENV).is_none() {
        return;
    }
    write(Stream::Stderr, format_args!("\r{}%", 50));
    write_line(Stream::Stderr, format_args!("Skipped {}", "x"));
}

/// `write` adds nothing after the text, for a progress line that a terminal
/// rewrites after a carriage return, and `write_line` adds a newline.
#[test]
fn write_adds_nothing_and_write_line_adds_a_newline() {
    let output = Command::new(env::current_exe().unwrap())
        .args([
            "child_writes_to_stderr",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(CHILD_ENV, "1")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "the child ended with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "\r50%Skipped x\n");
}
