mod support;

use std::io::Write;
use std::process::{Command, Stdio};

use support::fixture;

fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

fn compile_stdin(input: &str) -> std::process::Output {
    let mut child = Command::new(stk_bin())
        .args(["compile"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn ansi_heavy_fixture_strips_color_codes_but_preserves_the_message() {
    let fx = fixture::load("tests/fixtures/ansi_heavy.toml");
    let output = compile_stdin(fx.stdin.as_deref().unwrap());
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    fixture::assert_preserves(&stdout, &fx);
    assert!(
        !stdout.contains('\x1b'),
        "expected no raw ANSI escape bytes in compiled output, got: {stdout:?}"
    );
}

#[test]
fn repeated_retries_fixture_collapses_to_a_repeated_count() {
    let fx = fixture::load("tests/fixtures/repeated_retries.toml");
    let output = compile_stdin(fx.stdin.as_deref().unwrap());
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    fixture::assert_preserves(&stdout, &fx);
    assert_eq!(
        stdout.matches("retrying connection...").count(),
        1,
        "expected the repeated line to appear exactly once (collapsed), got: {stdout:?}"
    );
}

#[test]
fn long_lines_fixture_is_truncated_head_and_tail_preserved() {
    let fx = fixture::load("tests/fixtures/long_lines.toml");
    let output = compile_stdin(fx.stdin.as_deref().unwrap());
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    fixture::assert_preserves(&stdout, &fx);
    assert!(
        stdout.len() < fx.stdin.as_ref().unwrap().len(),
        "expected the long line to be shorter after compilation"
    );
}
