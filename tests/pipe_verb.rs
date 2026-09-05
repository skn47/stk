mod support;

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use support::rtk;

fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

fn run_piped_with_binary(bin: &str, args: &[&str], stdin_data: &[u8]) -> std::process::Output {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin_data).unwrap();
    child.wait_with_output().unwrap()
}

fn run_piped(args: &[&str], stdin_data: &[u8]) -> std::process::Output {
    run_piped_with_binary(stk_bin(), args, stdin_data)
}

#[test]
fn pipe_with_no_flags_defaults_to_passthrough() {
    let output = run_piped(&["pipe"], b"hello\nworld\n");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"hello\nworld\n");
}

#[test]
fn pipe_dash_f_ansi_strips_color_codes() {
    let output = run_piped(&["pipe", "-f", "ansi"], b"\x1b[31mred\x1b[0m plain");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"red plain");
}

#[test]
fn pipe_unknown_filter_fails_with_nonzero_exit() {
    let output = run_piped(&["pipe", "-f", "nonexistent"], b"hello");
    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}

#[test]
fn golden_comparison_passthrough_against_real_rtk_pipe() {
    if !rtk::is_available() {
        eprintln!("skipping: rtk is not installed in this environment");
        return;
    }
    let payload = b"\x1b[31mred\x1b[0m\nplain line\n";

    let via_rtk = run_piped_with_binary("rtk", &["pipe", "--passthrough"], payload);
    let via_stk = run_piped(&["pipe", "--passthrough"], payload);

    assert_eq!(via_stk.stdout, via_rtk.stdout);
    assert_eq!(via_stk.status.code(), via_rtk.status.code());
}

#[test]
fn golden_comparison_no_flags_defaults_to_passthrough_like_real_rtk() {
    if !rtk::is_available() {
        eprintln!("skipping: rtk is not installed in this environment");
        return;
    }
    let payload = b"plain output, no flags given\n";

    let via_rtk = run_piped_with_binary("rtk", &["pipe"], payload);
    let via_stk = run_piped(&["pipe"], payload);

    assert_eq!(via_stk.stdout, via_rtk.stdout);
    assert_eq!(via_stk.status.code(), via_rtk.status.code());
}

#[test]
fn broken_pipe_on_stdout_is_handled_gracefully_not_a_crash() {
    let mut child = Command::new(stk_bin())
        .args(["pipe", "--passthrough"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Close our read end of stdout entirely before the child ever gets to write, so its
    // write() calls hit EPIPE deterministically rather than depending on timing.
    drop(child.stdout.take());

    let mut stdin = child.stdin.take().unwrap();
    let payload = "x".repeat(1_000_000);
    // This write can itself fail once the child has exited; that's fine, we only care
    // that `stk` itself didn't crash or hang.
    let _ = stdin.write_all(payload.as_bytes());
    drop(stdin);

    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("stk pipe did not exit within 5s after its stdout was closed early");
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            status.signal(),
            None,
            "stk crashed via a signal instead of handling the broken pipe gracefully"
        );
    }
}
