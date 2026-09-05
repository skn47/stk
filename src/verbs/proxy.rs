use std::io::Write;

use crate::capture::executor::CommandExecutor;
use crate::history::{HistoryStore, UsageEntry};
use crate::verbs::passthrough;

/// `stk proxy [ARGS...]`: byte-identical passthrough like `run`, but silently records usage.
pub fn dispatch(
    args: &[String],
    stderr: &mut dyn Write,
    executor: &dyn CommandExecutor,
    history: &dyn HistoryStore,
) -> i32 {
    let Some((command, cmd_args)) = args.split_first() else {
        return passthrough::no_command_error("proxy", stderr);
    };

    // Recorded before executing so a crashed/killed child still leaves a usage trace, and
    // a failure here never blocks the passthrough itself (proxy is silent either way).
    let _ = history.record(&UsageEntry::now("proxy", command, cmd_args));

    match executor.execute_inherited(command, cmd_args) {
        Ok(info) => info.to_process_exit_code(),
        Err(err) => passthrough::exec_error(command, &err, stderr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::executor::{ExitInfo, FakeExecutor};
    use crate::history::FakeHistoryStore;

    #[test]
    fn records_usage_and_executes_the_command() {
        let executor = FakeExecutor::new();
        let history = FakeHistoryStore::new();
        let mut stderr = Vec::new();

        dispatch(
            &["echo".to_string(), "hi".to_string()],
            &mut stderr,
            &executor,
            &history,
        );

        assert_eq!(
            executor.invocations(),
            vec![("echo".to_string(), vec!["hi".to_string()])]
        );
        let entries = history.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].verb, "proxy");
        assert_eq!(entries[0].command, "echo");
        assert_eq!(entries[0].args, vec!["hi".to_string()]);
    }

    #[test]
    fn no_dash_c_support_positional_args_only() {
        // Unlike `run`, real `rtk proxy` has no -c flag: "-c" here is just an ordinary
        // (if unusual) argv[0] to execute, not a shell-string indicator.
        let executor = FakeExecutor::new();
        let history = FakeHistoryStore::new();
        let mut stderr = Vec::new();

        dispatch(
            &["-c".to_string(), "echo hi".to_string()],
            &mut stderr,
            &executor,
            &history,
        );

        assert_eq!(
            executor.invocations(),
            vec![("-c".to_string(), vec!["echo hi".to_string()])]
        );
    }

    #[test]
    fn no_command_is_a_usage_error_and_nothing_is_recorded_or_spawned() {
        let executor = FakeExecutor::new();
        let history = FakeHistoryStore::new();
        let mut stderr = Vec::new();

        let exit_code = dispatch(&[], &mut stderr, &executor, &history);

        assert_ne!(exit_code, 0);
        assert!(executor.invocations().is_empty());
        assert!(history.entries().is_empty());
    }

    #[test]
    fn exit_code_reflects_signal_termination_as_128_plus_signal() {
        let executor = FakeExecutor::new().with_inherited_response(ExitInfo {
            exit_code: 0,
            terminating_signal: Some(15),
        });
        let history = FakeHistoryStore::new();
        let mut stderr = Vec::new();

        let exit_code = dispatch(
            &["sleep".to_string(), "30".to_string()],
            &mut stderr,
            &executor,
            &history,
        );

        assert_eq!(exit_code, 128 + 15);
    }

    #[test]
    fn exit_code_reflects_normal_exit_when_no_signal() {
        let executor = FakeExecutor::new().with_inherited_response(ExitInfo {
            exit_code: 7,
            terminating_signal: None,
        });
        let history = FakeHistoryStore::new();
        let mut stderr = Vec::new();

        let exit_code = dispatch(&["false".to_string()], &mut stderr, &executor, &history);

        assert_eq!(exit_code, 7);
    }
}
