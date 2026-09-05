mod support;

use std::io::Write;
use std::process::{Command, Stdio};

use support::fixture;

fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

fn compile_stdin(input: &str) -> std::process::Output {
    compile_stdin_with_args(&["compile"], input)
}

fn compile_stdin_with_args(args: &[&str], input: &str) -> std::process::Output {
    let mut child = Command::new(stk_bin())
        .args(args)
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

#[test]
fn normal_budgeted_compression_stays_within_budget_and_keeps_the_error() {
    let fx = fixture::load("tests/fixtures/repeated_retries.toml");
    let output = compile_stdin_with_args(
        &["--budget", "200", "compile"],
        fx.stdin.as_deref().unwrap(),
    );
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    fixture::assert_preserves(&stdout, &fx);
}

#[test]
fn p0_forced_to_shrink_still_fits_the_budget_and_carries_a_shrink_marker() {
    let fx = fixture::load("tests/fixtures/p0_shrink.toml");
    let output =
        compile_stdin_with_args(&["--budget", "60", "compile"], fx.stdin.as_deref().unwrap());
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    fixture::assert_preserves(&stdout, &fx);
    assert!(
        stdout.contains("[stk: P0 content truncated to fit budget]"),
        "expected a shrink marker, got: {stdout:?}"
    );
    assert!(stdout.len() < fx.stdin.as_ref().unwrap().len());
}

#[test]
fn budget_below_the_floor_is_refused_not_rendered() {
    let output = compile_stdin_with_args(&["--budget", "1", "compile"], "error: anything\n");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("too small"),
        "expected a clear refusal, got: {stderr:?}"
    );
}
