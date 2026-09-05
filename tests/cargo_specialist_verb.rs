mod support;

use std::process::Command;

use support::rtk;

fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

fn fixture_dir(name: &str) -> String {
    format!(
        "{}/tests/fixtures/cargo_projects/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn type_error_fixture_preserves_the_error_code_and_location() {
    let output = Command::new(stk_bin())
        .current_dir(fixture_dir("type_error"))
        .args(["cargo", "check"])
        .output()
        .unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(!output.status.success());
    assert!(
        combined.contains("E0308"),
        "expected the error code, got: {combined:?}"
    );
    assert!(
        combined.contains("src/main.rs:6:30"),
        "expected the file:line:column location, got: {combined:?}"
    );
}

#[test]
fn type_error_fixture_under_budget_still_preserves_the_error_code_and_location() {
    let output = Command::new(stk_bin())
        .current_dir(fixture_dir("type_error"))
        .args(["--budget", "150", "cargo", "check"])
        .output()
        .unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(combined.contains("E0308"));
    assert!(combined.contains("src/main.rs:6:30"));
    assert!(
        combined.contains("[stk: omitted"),
        "expected the build-phase noise to actually be omitted under a tight budget, got: {combined:?}"
    );
}

#[test]
fn passing_build_with_warning_compresses_away_routine_success_output() {
    let output = Command::new(stk_bin())
        .current_dir(fixture_dir("passing_with_warning"))
        .args(["--budget", "80", "cargo", "build"])
        .output()
        .unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(output.status.success());
    assert!(
        !combined.contains("Compiling") && !combined.contains("Finished"),
        "expected routine build-phase noise to be compressed away, got: {combined:?}"
    );
}

#[test]
fn golden_comparison_retains_at_least_the_mandatory_evidence_real_rtk_does() {
    if !rtk::is_available() {
        eprintln!("skipping: rtk is not installed in this environment");
        return;
    }
    let dir = fixture_dir("type_error");

    let via_rtk = Command::new("rtk")
        .current_dir(&dir)
        .args(["cargo", "check"])
        .output()
        .unwrap();
    let via_stk = Command::new(stk_bin())
        .current_dir(&dir)
        .args(["cargo", "check"])
        .output()
        .unwrap();

    let rtk_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&via_rtk.stdout),
        String::from_utf8_lossy(&via_rtk.stderr)
    );
    let stk_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&via_stk.stdout),
        String::from_utf8_lossy(&via_stk.stderr)
    );

    assert!(
        rtk_combined.contains("E0308"),
        "sanity check: rtk itself should show the error code"
    );
    assert!(stk_combined.contains("E0308"));
    assert!(stk_combined.contains("src/main.rs:6:30"));
}
