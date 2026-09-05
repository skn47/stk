mod support;

use std::io::Write;
use std::process::{Command, Stdio};

use stk::capture::executor::{ExecutionResult, FakeExecutor};
use stk::history::FakeHistoryStore;

fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

/// `log`/`diff`/`json` take a file or stdin (no command execution) -- spawn the real
/// binary, matching the established convention for verbs that never touch CommandExecutor.
fn run_piped(args: &[&str], stdin_data: &[u8]) -> std::process::Output {
    let mut child = Command::new(stk_bin())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin_data).unwrap();
    child.wait_with_output().unwrap()
}

/// `err`/`summary` run a command -- exercised in-process via `cli::run` with a
/// `FakeExecutor`, matching how `compile`/specialist dispatch are tested.
fn run_with_fake_command(args: &[&str], executor: &FakeExecutor) -> (i32, String, String) {
    let history = FakeHistoryStore::new();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let exit_code = stk::cli::run(
        &args,
        std::io::empty(),
        &mut stdout,
        &mut stderr,
        executor,
        &history,
    );
    (
        exit_code,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

#[test]
fn log_filters_and_deduplicates_stdin() {
    let output = run_piped(&["log"], b"a\na\na\nerror: bad\n");
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    assert!(stdout.contains("a [repeated 3x]"));
    assert!(stdout.contains("error: bad"));
}

#[test]
fn err_isolates_errors_and_warnings_from_mixed_output() {
    let executor = FakeExecutor::new().with_response(ExecutionResult {
        exit_code: 1,
        terminating_signal: None,
        stdout: b"Compiling foo\nerror: boom\nFinished\n".to_vec(),
        stderr: b"warning: careful\nnote: fyi\n".to_vec(),
    });

    let (exit_code, stdout, _) = run_with_fake_command(&["err", "cargo", "check"], &executor);

    assert_eq!(exit_code, 1);
    assert!(stdout.contains("error: boom"));
    assert!(stdout.contains("warning: careful"));
    assert!(!stdout.contains("Compiling"));
    assert!(!stdout.contains("note: fyi"));
}

#[test]
fn summary_reports_counts_and_the_first_error() {
    let executor = FakeExecutor::new().with_response(ExecutionResult {
        exit_code: 1,
        terminating_signal: None,
        stdout: b"error: first problem\nerror: second problem\nwarning: heads up\n".to_vec(),
        stderr: Vec::new(),
    });

    let (exit_code, stdout, _) = run_with_fake_command(&["summary", "cargo", "test"], &executor);

    assert_eq!(exit_code, 1);
    assert!(stdout.contains("2 errors"));
    assert!(stdout.contains("1 warning"));
    assert!(stdout.contains("error: first problem"));
    assert!(!stdout.contains("second problem"));
}

#[test]
fn diff_condenses_to_hunk_headers_and_changed_lines() {
    let diff_text = "diff --git a/foo b/foo\nindex 111..222 100644\n--- a/foo\n+++ b/foo\n@@ -1,2 +1,2 @@\n context line\n-old line\n+new line\n";
    let output = run_piped(&["diff"], diff_text.as_bytes());
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    assert!(stdout.contains("diff --git a/foo b/foo"));
    assert!(stdout.contains("@@ -1,2 +1,2 @@"));
    assert!(stdout.contains("-old line"));
    assert!(stdout.contains("+new line"));
    assert!(!stdout.contains("context line"));
    assert!(!stdout.contains("index 111"));
}

#[test]
fn json_compacts_by_default_and_strips_values_with_keys_only() {
    let dir = std::env::temp_dir().join(format!(
        "stk-json-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("data.json");
    std::fs::write(&file, "{\n  \"a\": 1,\n  \"b\": [1, 2, 3]\n}\n").unwrap();

    let compact = run_piped(&["json", file.to_str().unwrap()], b"");
    let compact_stdout = String::from_utf8(compact.stdout).unwrap();
    assert!(compact.status.success());
    assert_eq!(compact_stdout.trim(), r#"{"a":1,"b":[1,2,3]}"#);

    let keys_only = run_piped(&["json", file.to_str().unwrap(), "--keys-only"], b"");
    let keys_only_stdout = String::from_utf8(keys_only.stdout).unwrap();
    assert_eq!(
        keys_only_stdout.trim(),
        r#"{"a":null,"b":[null,null,null]}"#
    );
}
