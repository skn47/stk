use std::io::{ErrorKind, Read, Write};

use crate::capture::executor::CommandExecutor;
use crate::fastpath;

/// `stk compile [CMD [ARGS...]]`: the generic fast-path pipeline. Runs a command
/// (buffered, unlike `run`/`proxy`) or, given none, reads stdin instead. These are
/// deliberately alternatives, not combinable: when a command is given, `stk`'s own
/// stdin is not forwarded to it (matching the design doc's "reads stdin *instead of*
/// executing a command" framing) -- a command that itself wants piped stdin should use
/// `stk run`/`stk proxy` instead of `compile`.
pub fn dispatch(
    args: &[String],
    mut stdin: impl Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    executor: &dyn CommandExecutor,
) -> i32 {
    match args.split_first() {
        Some((command, cmd_args)) => match executor.execute(command, cmd_args) {
            Ok(result) => {
                if let WriteOutcome::Failed(err) =
                    write_output(stdout, &fastpath::run(&result.stdout))
                {
                    let _ = writeln!(stderr, "stk: failed to write output: {err}");
                    return 1;
                }
                if let WriteOutcome::Failed(err) =
                    write_output(stderr, &fastpath::run(&result.stderr))
                {
                    let _ = writeln!(stderr, "stk: failed to write output: {err}");
                    return 1;
                }
                // A broken pipe on either stream above is a normal early pipeline
                // shutdown (e.g. `| head`), not a reason to hide the child's real
                // outcome -- a script checking $?/PIPESTATUS still needs the truth.
                result.exit_info().to_process_exit_code()
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
            match write_output(stdout, &fastpath::run(&input)) {
                WriteOutcome::Failed(err) => {
                    let _ = writeln!(stderr, "stk: failed to write output: {err}");
                    1
                }
                WriteOutcome::Ok | WriteOutcome::BrokenPipe => 0,
            }
        }
    }
}

enum WriteOutcome {
    Ok,
    BrokenPipe,
    Failed(std::io::Error),
}

fn write_output(sink: &mut dyn Write, data: &[u8]) -> WriteOutcome {
    match sink.write_all(data) {
        Ok(()) => WriteOutcome::Ok,
        Err(err) if err.kind() == ErrorKind::BrokenPipe => WriteOutcome::BrokenPipe,
        Err(err) => WriteOutcome::Failed(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::executor::{ExecutionResult, FakeExecutor};

    fn run(args: &[&str], stdin: &str, executor: &FakeExecutor) -> (i32, String, String) {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let exit_code = dispatch(&args, stdin.as_bytes(), &mut stdout, &mut stderr, executor);
        (
            exit_code,
            String::from_utf8(stdout).unwrap(),
            String::from_utf8(stderr).unwrap(),
        )
    }

    #[test]
    fn no_args_reads_and_compiles_stdin() {
        let executor = FakeExecutor::new();
        let (exit_code, stdout, _) = run(&[], "retry\nretry\nretry\n", &executor);

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

        let (exit_code, stdout, stderr) = run(&["cargo", "check"], "", &executor);

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

        let (exit_code, _, _) = run(&["false"], "", &executor);
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
        );

        assert_eq!(
            exit_code, 101,
            "a broken pipe on stdout must not overwrite the real child exit code"
        );
    }
}
