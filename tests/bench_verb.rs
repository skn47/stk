mod support;

use std::process::Command;

use support::rtk;

fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

#[test]
fn wrong_args_is_a_usage_error() {
    let output = Command::new(stk_bin())
        .args(["bench", "--against", "something-else"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage"));
}

#[test]
fn no_args_is_a_usage_error() {
    let output = Command::new(stk_bin()).args(["bench"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

/// Excludes `rtk` from `PATH` (rather than relying on it happening to be absent from
/// the test environment) so this case is deterministic regardless of whether `rtk` is
/// actually installed here.
#[test]
fn reports_gracefully_and_exits_zero_when_rtk_is_not_on_path() {
    let dir = tempfile_free_bin_dir();
    let output = Command::new(stk_bin())
        .args(["bench", "--against", "rtk"])
        .env("PATH", &dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("not installed"));
}

fn tempfile_free_bin_dir() -> std::path::PathBuf {
    // A PATH directory that exists but contains no `rtk` (or anything else): guarantees
    // `Command::new("rtk")` fails to spawn, without disturbing the real PATH otherwise.
    let dir = std::env::temp_dir().join(format!(
        "stk-bench-test-empty-path-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The real end-to-end regression gate: every golden comparison built up across this
/// codebase's own verb-specific test files must still pass as `stk bench --against rtk`.
#[test]
fn all_cases_pass_against_the_real_installed_rtk() {
    if !rtk::is_available() {
        eprintln!("skipping: rtk is not installed in this environment");
        return;
    }
    let output = Command::new(stk_bin())
        .args(["bench", "--against", "rtk"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "bench reported a failure: {stdout}"
    );
    assert!(!stdout.contains("[FAIL]"), "got: {stdout}");
    assert!(stdout.contains("passed"), "got: {stdout}");
}
