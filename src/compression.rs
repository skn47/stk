use std::io::{ErrorKind, Write};

use crate::budget::selector::{self, BudgetError};
use crate::budget::tokenizer::ApproximateCounter;
use crate::capture::executor::CommandExecutor;
use crate::chunk::Classifier;
use crate::fastpath;

/// Shared by `stk compile` (generic classifier) and specialist dispatch (`stk cargo
/// ...`, its own classifier): executes `command`, applies the fast path or (if `budget`
/// is given) the `Budget` Selection Algorithm, and writes the result.
pub fn execute_and_compress(
    command: &str,
    cmd_args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    executor: &dyn CommandExecutor,
    budget: Option<usize>,
    classify: Classifier,
) -> i32 {
    match executor.execute(command, cmd_args) {
        Ok(result) => {
            let outcome = match budget {
                None => write_output(stdout, &fastpath::run(&result.stdout))
                    .and_then(|()| write_output(stderr, &fastpath::run(&result.stderr))),
                Some(budget) => write_budgeted(
                    stdout,
                    stderr,
                    &result.stdout,
                    &result.stderr,
                    budget,
                    classify,
                ),
            };
            match outcome {
                Ok(()) => result.exit_info().to_process_exit_code(),
                Err(WriteOutcome::Failed(err)) => {
                    let _ = writeln!(stderr, "stk: failed to write output: {err}");
                    1
                }
                // A broken pipe is a normal early pipeline shutdown (e.g. `| head`), not
                // a reason to hide the child's real outcome -- a script checking
                // $?/PIPESTATUS still needs the truth.
                Err(WriteOutcome::BrokenPipe) => result.exit_info().to_process_exit_code(),
                Err(WriteOutcome::AlreadyReported(code)) => code,
            }
        }
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to run '{command}': {err}");
            1
        }
    }
}

/// Shared stdin-reading half of `stk compile` (no command given): applies the fast path
/// or `Budget` Selection Algorithm to piped input directly, with no exit code to preserve.
pub fn compress_stdin(
    input: &[u8],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    budget: Option<usize>,
    classify: Classifier,
) -> i32 {
    let outcome = match budget {
        None => write_output(stdout, &fastpath::run(input)),
        Some(budget) => write_budgeted(stdout, stderr, input, &[], budget, classify),
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

fn write_budgeted(
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    raw_stdout: &[u8],
    raw_stderr: &[u8],
    budget: usize,
    classify: Classifier,
) -> Result<(), WriteOutcome> {
    let stdout_lines = fastpath::lines_from(raw_stdout);
    let stderr_lines = fastpath::lines_from(raw_stderr);

    match selector::select(
        &stdout_lines,
        &stderr_lines,
        budget,
        &ApproximateCounter,
        classify,
    ) {
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

pub(crate) enum WriteOutcome {
    BrokenPipe,
    Failed(std::io::Error),
    /// The failure was already reported to stderr (by the caller's own, more specific
    /// message) -- the caller should use this exit code without printing another,
    /// generic "failed to write output" line for the same failure.
    AlreadyReported(i32),
}

pub(crate) fn write_output(sink: &mut dyn Write, data: &[u8]) -> Result<(), WriteOutcome> {
    match sink.write_all(data) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::BrokenPipe => Err(WriteOutcome::BrokenPipe),
        Err(err) => Err(WriteOutcome::Failed(err)),
    }
}
