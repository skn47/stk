use stk::capture::executor::FakeExecutor;
use stk::cli;
use stk::history::FakeHistoryStore;

#[test]
fn unrecognized_command_is_rejected_with_nonzero_exit() {
    let executor = FakeExecutor::new();
    let history = FakeHistoryStore::new();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let exit_code = cli::run(
        &["frobnicate".to_string()],
        std::io::empty(),
        &mut stdout,
        &mut stderr,
        &executor,
        &history,
    );

    assert_ne!(exit_code, 0);
    assert!(stdout.is_empty());
    let stderr = String::from_utf8(stderr).unwrap();
    assert!(
        stderr.contains("unsupported") || stderr.contains("frobnicate"),
        "expected an unsupported-command error mentioning the command, got: {stderr}"
    );
}

#[test]
fn no_command_given_is_also_rejected() {
    let executor = FakeExecutor::new();
    let history = FakeHistoryStore::new();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let exit_code = cli::run(
        &[],
        std::io::empty(),
        &mut stdout,
        &mut stderr,
        &executor,
        &history,
    );

    assert_ne!(exit_code, 0);
    assert!(!String::from_utf8(stderr).unwrap().is_empty());
}

#[test]
fn no_child_process_is_spawned_for_an_unsupported_command() {
    let executor = FakeExecutor::new();
    let history = FakeHistoryStore::new();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    cli::run(
        &["cargo".to_string(), "check".to_string()],
        std::io::empty(),
        &mut stdout,
        &mut stderr,
        &executor,
        &history,
    );

    assert_eq!(
        executor.invocations().len(),
        0,
        "ticket 01 has no registered specialists yet, so `stk cargo check` must not reach the executor"
    );
}
