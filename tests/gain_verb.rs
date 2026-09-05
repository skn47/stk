mod support;

use std::path::PathBuf;
use std::process::Command;

use stk::capture::executor::{ExecutionResult, FakeExecutor};
use stk::history::FakeHistoryStore;

fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

fn run_in_process(
    args: &[&str],
    executor: &FakeExecutor,
    history: &FakeHistoryStore,
) -> (i32, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let exit_code = stk::cli::run(
        &args,
        std::io::empty(),
        &mut stdout,
        &mut stderr,
        executor,
        history,
    );
    (
        exit_code,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

/// End-to-end: a verb that actually compresses something (via `compression.rs`'s shared
/// path) records real savings that `stk gain` then reports -- not just gain's own unit
/// tests against a hand-built history, but the full recording pipeline wired together.
#[test]
fn compile_records_savings_that_gain_then_summarizes() {
    let executor = FakeExecutor::new().with_response(ExecutionResult {
        exit_code: 0,
        terminating_signal: None,
        stdout: b"retry\nretry\nretry\nretry\nretry\n".to_vec(),
        stderr: Vec::new(),
    });
    let history = FakeHistoryStore::new();

    let (code, _, _) = run_in_process(&["compile", "cargo", "check"], &executor, &history);
    assert_eq!(code, 0);

    let (code, stdout, _) = run_in_process(&["gain"], &executor, &history);
    assert_eq!(code, 0);
    assert!(stdout.contains("Total commands:  1"));
    assert!(!stdout.contains("No tracking data"));
}

/// A verb that never compresses anything (`stk proxy`) records plain usage but
/// contributes nothing to `gain`'s savings summary.
#[test]
fn proxy_usage_alone_does_not_count_as_savings_data() {
    let executor = FakeExecutor::new().with_response(ExecutionResult {
        exit_code: 0,
        terminating_signal: None,
        stdout: Vec::new(),
        stderr: Vec::new(),
    });
    let history = FakeHistoryStore::new();

    let (code, _, _) = run_in_process(&["proxy", "echo", "hi"], &executor, &history);
    assert_eq!(code, 0);

    let (code, stdout, _) = run_in_process(&["gain"], &executor, &history);
    assert_eq!(code, 0);
    assert!(stdout.contains("No tracking data yet"));
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "stk-gain-test-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Real filesystem/env-backed history (`FileHistoryStore`), spawning the real binary
/// with an isolated `HOME` -- `stk read` (which never touches `CommandExecutor`) records
/// savings that a later `stk gain` process, sharing the same `HOME`, can read back.
#[test]
fn a_real_file_backed_history_persists_savings_across_separate_invocations() {
    let home = temp_dir("filestore-home");
    let cwd = temp_dir("filestore-cwd");
    std::fs::write(cwd.join("f.txt"), "hello world\n").unwrap();

    let read_output = Command::new(stk_bin())
        .args(["read", "f.txt"])
        .current_dir(&cwd)
        .env_remove("XDG_CACHE_HOME")
        .env("HOME", &home)
        .output()
        .unwrap();
    assert!(read_output.status.success());

    let gain_output = Command::new(stk_bin())
        .args(["gain"])
        .current_dir(&cwd)
        .env_remove("XDG_CACHE_HOME")
        .env("HOME", &home)
        .output()
        .unwrap();
    assert!(gain_output.status.success());
    let stdout = String::from_utf8(gain_output.stdout).unwrap();
    assert!(stdout.contains("Total commands:  1"), "got: {stdout:?}");
}

/// Golden-comparison: for every flag in the documented subset, `stk gain` doesn't error,
/// and (only if the real `rtk` is installed) neither does `rtk gain` with the same flag
/// in its own isolated environment -- proving the flag set itself is accepted by both,
/// not that their output is byte-identical (impossible given different tokenizers/data).
#[test]
fn gain_accepts_the_documented_rtk_flag_subset() {
    let flag_sets: &[&[&str]] = &[
        &[],
        &["-p"],
        &["-H"],
        &["-g"],
        &["-q"],
        &["-q", "-t", "pro"],
        &["-q", "-t", "5x"],
        &["-q", "-t", "20x"],
        &["-d"],
        &["-w"],
        &["-m"],
        &["-a"],
        &["-f", "text"],
        &["-f", "json"],
        &["-f", "csv"],
        &["-F"],
        &["--reset"],
    ];

    let rtk_available = support::rtk::is_available();

    for flags in flag_sets {
        let home = temp_dir("flagset-stk-home");
        let cwd = temp_dir("flagset-stk-cwd");
        let output = Command::new(stk_bin())
            .arg("gain")
            .args(*flags)
            .current_dir(&cwd)
            .env_remove("XDG_CACHE_HOME")
            .env("HOME", &home)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stk gain {flags:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        if rtk_available {
            let rtk_home = temp_dir("flagset-rtk-home");
            let rtk_cwd = temp_dir("flagset-rtk-cwd");
            let rtk_output = Command::new("rtk")
                .arg("gain")
                .args(*flags)
                .current_dir(&rtk_cwd)
                .env("HOME", &rtk_home)
                .output()
                .unwrap();
            assert!(
                rtk_output.status.success(),
                "rtk gain {flags:?} failed: {}",
                String::from_utf8_lossy(&rtk_output.stderr)
            );
        }
    }

    if !rtk_available {
        eprintln!("skipping rtk-side comparison: rtk is not installed in this environment");
    }
}
