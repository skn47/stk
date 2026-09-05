use std::io::Write;

use crate::capture::executor::{CommandExecutor, ExitInfo};

/// `stk run [ARGS...]` / `stk run -c COMMAND`: raw passthrough, no filtering, no tracking.
pub fn dispatch(args: &[String], stderr: &mut dyn Write, executor: &dyn CommandExecutor) -> i32 {
    let (command, cmd_args) = match parse(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            let _ = writeln!(stderr, "stk: {message}");
            return 2;
        }
    };

    match executor.execute_inherited(&command, &cmd_args) {
        Ok(info) => exit_code_for(info),
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to run '{command}': {err}");
            1
        }
    }
}

fn parse(args: &[String]) -> Result<(String, Vec<String>), &'static str> {
    match args.first().map(String::as_str) {
        Some("-c") | Some("--command") => match args.get(1) {
            Some(command_string) => Ok((
                "sh".to_string(),
                vec!["-c".to_string(), command_string.clone()],
            )),
            None => Err("run -c requires a command string"),
        },
        Some(_) => Ok((args[0].clone(), args[1..].to_vec())),
        None => Err("run requires a command"),
    }
}

fn exit_code_for(info: ExitInfo) -> i32 {
    match info.terminating_signal {
        Some(signal) => 128 + signal,
        None => info.exit_code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::executor::FakeExecutor;

    #[test]
    fn positional_args_are_executed_directly_not_through_a_shell() {
        let executor = FakeExecutor::new();
        let mut stderr = Vec::new();

        dispatch(
            &["echo".to_string(), "hi".to_string()],
            &mut stderr,
            &executor,
        );

        assert_eq!(
            executor.invocations(),
            vec![("echo".to_string(), vec!["hi".to_string()])]
        );
    }

    #[test]
    fn dash_c_form_runs_through_sh() {
        let executor = FakeExecutor::new();
        let mut stderr = Vec::new();

        dispatch(
            &["-c".to_string(), "echo hi".to_string()],
            &mut stderr,
            &executor,
        );

        assert_eq!(
            executor.invocations(),
            vec![(
                "sh".to_string(),
                vec!["-c".to_string(), "echo hi".to_string()]
            )]
        );
    }

    #[test]
    fn no_command_is_a_usage_error_and_nothing_is_spawned() {
        let executor = FakeExecutor::new();
        let mut stderr = Vec::new();

        let exit_code = dispatch(&[], &mut stderr, &executor);

        assert_ne!(exit_code, 0);
        assert!(executor.invocations().is_empty());
        assert!(!String::from_utf8(stderr).unwrap().is_empty());
    }

    #[test]
    fn exit_code_reflects_signal_termination_as_128_plus_signal() {
        let executor = FakeExecutor::new().with_inherited_response(ExitInfo {
            exit_code: 0,
            terminating_signal: Some(15),
        });
        let mut stderr = Vec::new();

        let exit_code = dispatch(
            &["sleep".to_string(), "30".to_string()],
            &mut stderr,
            &executor,
        );

        assert_eq!(exit_code, 128 + 15);
    }

    #[test]
    fn exit_code_reflects_normal_exit_when_no_signal() {
        let executor = FakeExecutor::new().with_inherited_response(ExitInfo {
            exit_code: 7,
            terminating_signal: None,
        });
        let mut stderr = Vec::new();

        let exit_code = dispatch(&["false".to_string()], &mut stderr, &executor);

        assert_eq!(exit_code, 7);
    }
}
