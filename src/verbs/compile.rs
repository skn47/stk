use std::io::{ErrorKind, Read, Write};

use crate::budget::selector::{self, BudgetError};
use crate::budget::tokenizer::ApproximateCounter;
use crate::capture::executor::CommandExecutor;
use crate::fastpath;

/// `stk compile [CMD [ARGS...]]`: runs a command (buffered) or, given none, reads stdin.
/// Deliberately not combinable: a given command's stdin is never forwarded -- use
/// `stk run`/`stk proxy` for a command that itself needs piped stdin.
pub fn dispatch(
    args: &[String],
    mut stdin: impl Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    executor: &dyn CommandExecutor,
    budget: Option<usize>,
) -> i32 {
    match args.split_first() {
        Some((command, cmd_args)) => match executor.execute(command, cmd_args) {
            Ok(result) => {
                let outcome = match budget {
                    None => write_output(stdout, &fastpath::run(&result.stdout))
                        .and_then(|()| write_output(stderr, &fastpath::run(&result.stderr))),
                    Some(budget) => {
                        write_budgeted(stdout, stderr, &result.stdout, &result.stderr, budget)
                    }
                };
                match outcome {
                    Ok(()) => result.exit_info().to_process_exit_code(),
                    Err(WriteOutcome::Failed(err)) => {
                        let _ = writeln!(stderr, "stk: failed to write output: {err}");
                        1
                    }
                    // A broken pipe is a normal early pipeline shutdown (e.g. `| head`),
                    // not a reason to hide the child's real outcome -- a script checking
                    // $?/PIPESTATUS still needs the truth.
                    Err(WriteOutcome::BrokenPipe) => result.exit_info().to_process_exit_code(),
                    Err(WriteOutcome::AlreadyReported(code)) => code,
                }
            }
            Err(err) => {
                let _ = writeln!(stderr, "stk: failed to run '{command}': {err}");
                1
            }
        },
        None => {
            let mut input = Vec::new();
            if let Err(err) = stdin.read_to_end(&mut input) {
                let _ = writeln!(stderr, "stk: failed to read stdin: {err}");
                return 1;
            }
            let outcome = match budget {
                None => write_output(stdout, &fastpath::run(&input)),
                Some(budget) => write_budgeted(stdout, stderr, &input, &[], budget),
            };
            match outcome {
                Ok(()) | Err(WriteOutcome::BrokenPipe) => 0,
                Err(WriteOutcome::Failed(err)) => {
                    let _ = writeln!(stderr, "stk: failed to write output: {err}");
                    1
                }
                Err(WriteOutcome::AlreadyReported(code)) => code,
            }
        }
    }
}

/// Runs the Budget Selection Algorithm over `stdout`/`stderr` and writes the result.
fn write_budgeted(
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    raw_stdout: &[u8],
    raw_stderr: &[u8],
    budget: usize,
) -> Result<(), WriteOutcome> {
    let stdout_lines = fastpath::lines_from(raw_stdout);
    let stderr_lines = fastpath::lines_from(raw_stderr);

    match selector::select(&stdout_lines, &stderr_lines, budget, &ApproximateCounter) {
        Ok(output) => {
            write_output(stdout, &output.stdout)?;
            write_output(stderr, &output.stderr)
        }
        Err(BudgetError::TooSmall { budget, minimum }) => {
            let _ = writeln!(
                stderr,
                "stk: --budget {budget} is too small to render mandatory content (minimum: {minimum})"
            );
            Err(WriteOutcome::AlreadyReported(2))
        }
        Err(BudgetError::Internal) => {
            let _ = writeln!(
                stderr,
                "stk: internal error: render exceeded budget after dropping all optional content"
            );
            Err(WriteOutcome::AlreadyReported(1))
        }
    }
}

enum WriteOutcome {
    BrokenPipe,
    Failed(std::io::Error),
    /// The failure was already reported to stderr (by `write_budgeted`'s own, more
    /// specific message) -- the caller should use this exit code without printing
    /// another, generic "failed to write output" line for the same failure.
    AlreadyReported(i32),
}

fn write_output(sink: &mut dyn Write, data: &[u8]) -> Result<(), WriteOutcome> {
    match sink.write_all(data) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::BrokenPipe => Err(WriteOutcome::BrokenPipe),
        Err(err) => Err(WriteOutcome::Failed(err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::executor::{ExecutionResult, FakeExecutor};

    fn run(
        args: &[&str],
        stdin: &str,
        executor: &FakeExecutor,
        budget: Option<usize>,
    ) -> (i32, String, String) {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let exit_code = dispatch(
            &args,
            stdin.as_bytes(),
            &mut stdout,
            &mut stderr,
            executor,
            budget,
        );
        (
            exit_code,
            String::from_utf8(stdout).unwrap(),
            String::from_utf8(stderr).unwrap(),
        )
    }

    #[test]
    fn no_args_reads_and_compiles_stdin() {
        let executor = FakeExecutor::new();
        let (exit_code, stdout, _) = run(&[], "retry\nretry\nretry\n", &executor, None);

        assert_eq!(exit_code, 0);
        assert_eq!(stdout, "retry [repeated 3x]\n");
        assert!(executor.invocations().is_empty());
    }

    #[test]
    fn a_command_is_executed_via_the_buffered_executor_not_inherited() {
        let executor = FakeExecutor::new().with_response(ExecutionResult {
            exit_code: 0,
            terminating_signal: None,
            stdout: b"\x1b[31merror\x1b[0m\n".to_vec(),
            stderr: b"warn\nwarn\n".to_vec(),
        });

        let (exit_code, stdout, stderr) = run(&["cargo", "check"], "", &executor, None);

        assert_eq!(exit_code, 0);
        assert_eq!(stdout, "error\n");
        assert_eq!(stderr, "warn [repeated 2x]\n");
        assert_eq!(
            executor.invocations(),
            vec![("cargo".to_string(), vec!["check".to_string()])]
        );
    }

    #[test]
    fn exit_code_reflects_the_childs_exit_code() {
        let executor = FakeExecutor::new().with_response(ExecutionResult {
            exit_code: 3,
            terminating_signal: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
        });

        let (exit_code, _, _) = run(&["false"], "", &executor, None);
        assert_eq!(exit_code, 3);
    }

    struct BrokenPipeWriter;

    impl Write for BrokenPipeWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(ErrorKind::BrokenPipe, "pipe closed"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn broken_pipe_on_stdout_does_not_hide_the_childs_real_exit_code() {
        let executor = FakeExecutor::new().with_response(ExecutionResult {
            exit_code: 101,
            terminating_signal: None,
            stdout: b"some output\n".to_vec(),
            stderr: Vec::new(),
        });
        let mut stdout = BrokenPipeWriter;
        let mut stderr = Vec::new();

        let exit_code = dispatch(
            &["cargo".to_string(), "build".to_string()],
            std::io::empty(),
            &mut stdout,
            &mut stderr,
            &executor,
            None,
        );

        assert_eq!(
            exit_code, 101,
            "a broken pipe on stdout must not overwrite the real child exit code"
        );
    }

    #[test]
    fn a_budget_bounds_the_output_and_preserves_the_error() {
        let executor = FakeExecutor::new().with_response(ExecutionResult {
            exit_code: 0,
            terminating_signal: None,
            stdout: b"Compiling stk v0.1.0\nerror[E0308]: mismatched types\nwarning: unused\n"
                .to_vec(),
            stderr: Vec::new(),
        });

        let (exit_code, stdout, _) = run(&["cargo", "check"], "", &executor, Some(100));

        assert_eq!(exit_code, 0);
        assert!(stdout.contains("error[E0308]"));
    }

    #[test]
    fn a_budget_below_the_floor_is_refused_with_a_clear_error_and_nonzero_exit() {
        let executor = FakeExecutor::new().with_response(ExecutionResult {
            exit_code: 0,
            terminating_signal: None,
            stdout: b"error: anything\n".to_vec(),
            stderr: Vec::new(),
        });

        let (exit_code, stdout, stderr) = run(&["cargo", "check"], "", &executor, Some(1));

        assert_ne!(exit_code, 0);
        assert!(stdout.is_empty());
        assert!(stderr.contains("too small"));
        assert_eq!(
            stderr.lines().count(),
            1,
            "expected exactly one clear message, not also a generic \
             'failed to write output' line for the same rejection, got: {stderr:?}"
        );
    }

    #[test]
    fn budget_also_applies_when_reading_stdin() {
        let executor = FakeExecutor::new();
        let input = "error: important\n".to_string()
            + &"note: filler that does not matter at all here\n".repeat(5);

        let (exit_code, stdout, _) = run(&[], &input, &executor, Some(60));

        assert_eq!(exit_code, 0);
        assert!(stdout.contains("error: important"));
    }
}
