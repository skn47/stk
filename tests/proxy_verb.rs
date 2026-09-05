mod support;

use std::process::Command;

use support::{fixture, rtk};

fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

/// A fresh temp dir to point XDG_CACHE_HOME at, so tests never touch the developer's
/// real `~/.cache/stk/history.log`.
fn temp_cache_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "stk-proxy-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn proxy_is_byte_identical_to_running_the_command_directly() {
    let fx = fixture::load("tests/fixtures/echo_hello.toml");

    let direct = Command::new(&fx.command[0])
        .args(&fx.command[1..])
        .output()
        .unwrap();

    let mut stk_args = vec!["proxy".to_string()];
    stk_args.extend(fx.command.iter().cloned());
    let via_stk = Command::new(stk_bin())
        .args(&stk_args)
        .env("XDG_CACHE_HOME", temp_cache_dir())
        .output()
        .unwrap();

    assert_eq!(via_stk.stdout, direct.stdout);
    assert_eq!(via_stk.stderr, direct.stderr);
    assert_eq!(via_stk.status.code(), direct.status.code());
    assert!(
        via_stk.stderr.is_empty(),
        "proxy must never print a tracking summary"
    );
}

#[test]
fn proxy_records_usage_to_the_history_file() {
    let cache_dir = temp_cache_dir();

    let output = Command::new(stk_bin())
        .args(["proxy", "echo", "recorded"])
        .env("XDG_CACHE_HOME", &cache_dir)
        .output()
        .unwrap();
    assert!(output.status.success());

    let history_path = cache_dir.join("stk").join("history.log");
    let history = std::fs::read_to_string(&history_path)
        .unwrap_or_else(|e| panic!("expected {history_path:?} to exist: {e}"));

    let lines: Vec<&str> = history.lines().collect();
    assert_eq!(lines.len(), 1, "expected exactly one recorded entry");
    assert!(lines[0].contains("proxy"));
    assert!(lines[0].contains("echo"));
    assert!(lines[0].contains("recorded"));
}

#[test]
fn golden_comparison_against_real_rtk_proxy() {
    if !rtk::is_available() {
        eprintln!("skipping: rtk is not installed in this environment");
        return;
    }
    let fx = fixture::load("tests/fixtures/echo_hello.toml");

    let mut args = vec!["proxy".to_string()];
    args.extend(fx.command.iter().cloned());

    // Isolate both invocations' history writes from the developer's real ~/.cache/stk
    // (and whatever real rtk's own equivalent is) so running this test has no side effects.
    let via_rtk = Command::new("rtk")
        .args(&args)
        .env("XDG_CACHE_HOME", temp_cache_dir())
        .output()
        .unwrap();
    let via_stk = Command::new(stk_bin())
        .args(&args)
        .env("XDG_CACHE_HOME", temp_cache_dir())
        .output()
        .unwrap();

    assert_eq!(via_stk.stdout, via_rtk.stdout);
    assert_eq!(via_stk.status.code(), via_rtk.status.code());
}

#[test]
fn golden_comparison_dash_c_is_not_a_flag_for_proxy_unlike_run() {
    // Verified against the real rtk: `rtk proxy -c "echo hi"` tries to exec a literal
    // program named "-c" and fails (exit 1) -- proxy has no -c flag, unlike run.
    if !rtk::is_available() {
        eprintln!("skipping: rtk is not installed in this environment");
        return;
    }
    let args = ["proxy", "-c", "echo hi"];

    let via_rtk = Command::new("rtk")
        .args(args)
        .env("XDG_CACHE_HOME", temp_cache_dir())
        .output()
        .unwrap();
    let via_stk = Command::new(stk_bin())
        .args(args)
        .env("XDG_CACHE_HOME", temp_cache_dir())
        .output()
        .unwrap();

    assert_eq!(via_stk.status.code(), via_rtk.status.code());
    assert_ne!(via_stk.status.code(), Some(0));
}
