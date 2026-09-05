mod support;

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use support::{fixture, rtk};

fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

#[test]
fn run_is_byte_identical_to_running_the_command_directly() {
    let fx = fixture::load("tests/fixtures/echo_hello.toml");

    let direct = Command::new(&fx.command[0])
        .args(&fx.command[1..])
        .output()
        .unwrap();

    let mut stk_args = vec!["run".to_string()];
    stk_args.extend(fx.command.iter().cloned());
    let via_stk = Command::new(stk_bin()).args(&stk_args).output().unwrap();

    assert_eq!(via_stk.stdout, direct.stdout);
    assert_eq!(via_stk.stderr, direct.stderr);
    assert_eq!(via_stk.status.code(), direct.status.code());
    fixture::assert_preserves(&String::from_utf8_lossy(&via_stk.stdout), &fx);
}

#[test]
fn run_passes_stdin_through_to_the_child() {
    let mut child = Command::new(stk_bin())
        .args(["run", "cat"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"piped input\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert_eq!(output.stdout, b"piped input\n");
}

#[test]
fn run_dash_c_goes_through_a_shell() {
    let output = Command::new(stk_bin())
        .args(["run", "-c", "echo one && echo two"])
        .output()
        .unwrap();

    assert_eq!(output.stdout, b"one\ntwo\n");
}

#[test]
fn run_with_no_command_is_a_usage_error() {
    let output = Command::new(stk_bin()).arg("run").output().unwrap();
    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}

#[test]
fn golden_comparison_against_real_rtk_run() {
    if !rtk::is_available() {
        eprintln!("skipping: rtk is not installed in this environment");
        return;
    }
    let fx = fixture::load("tests/fixtures/echo_hello.toml");

    let mut args = vec!["run".to_string()];
    args.extend(fx.command.iter().cloned());

    let via_rtk = Command::new("rtk").args(&args).output().unwrap();
    let via_stk = Command::new(stk_bin()).args(&args).output().unwrap();

    assert_eq!(via_stk.stdout, via_rtk.stdout);
    assert_eq!(via_stk.status.code(), via_rtk.status.code());
}

#[test]
fn sigterm_is_forwarded_to_the_child_and_propagated_as_128_plus_signal() {
    let mut child = Command::new(stk_bin())
        .args(["run", "sleep", "30"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // Give `stk run` time to actually spawn `sleep` and install its signal forwarder
    // before we send the signal. A fixed sleep can't fully rule out a slow/loaded
    // machine still racing this, but 1s is a generous margin for what's microsecond-scale
    // work in practice.
    std::thread::sleep(Duration::from_secs(1));

    unsafe {
        libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
    }

    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("stk run did not exit within 5s of the child receiving SIGTERM — signal forwarding is broken");
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    assert_eq!(status.code(), Some(128 + libc::SIGTERM));
}
