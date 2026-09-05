use std::io::{ErrorKind, Write};

use crate::budget::selector::{self, BudgetError};
use crate::budget::tokenizer::ApproximateCounter;
use crate::capture::executor::CommandExecutor;
use crate::chunk::Classifier;
use crate::fastpath;
use crate::history::{record_savings, HistoryStore};

/// Shared by `stk compile` (generic classifier) and specialist dispatch (`stk cargo
/// ...`, its own classifier): executes `command`, applies the fast path or (if `budget`
/// is given) the `Budget` Selection Algorithm, writes the result, and records savings.
#[allow(clippy::too_many_arguments)]
pub fn execute_and_compress(
    verb: &str,
    command: &str,
    cmd_args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    executor: &dyn CommandExecutor,
    history: &dyn HistoryStore,
    budget: Option<usize>,
    classify: Classifier,
) -> i32 {
    match executor.execute(command, cmd_args) {
        Ok(result) => {
            let rendered = match budget {
                None => {
                    let out = fastpath::run(&result.stdout);
                    let err = fastpath::run(&result.stderr);
                    write_output(stdout, &out)
                        .and_then(|()| write_output(stderr, &err))
                        .map(|()| (out, err))
                }
                Some(budget) => write_budgeted(
                    stdout,
                    stderr,
                    &result.stdout,
                    &result.stderr,
                    budget,
                    classify,
                ),
            };
            match rendered {
                Ok((rendered_stdout, rendered_stderr)) => {
                    record_savings(
                        history,
                        &ApproximateCounter,
                        verb,
                        command,
                        cmd_args,
                        &combine_lossy(&result.stdout, &result.stderr),
                        &combine_lossy(&rendered_stdout, &rendered_stderr),
                    );
                    result.exit_info().to_process_exit_code()
                }
                Err(WriteOutcome::Failed(err)) => {
                    let _ = writeln!(stderr, "stk: failed to write output: {err}");
                    1
                }
                // A broken pipe is a normal early shutdown (e.g. `| head`), not a reason
                // to hide the child's real exit code -- but savings go unrecorded, since
                // what actually reached the pipe before it broke is unknown.
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
    verb: &str,
    input: &[u8],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    history: &dyn HistoryStore,
    budget: Option<usize>,
    classify: Classifier,
) -> i32 {
    let rendered = match budget {
        None => {
            let out = fastpath::run(input);
            write_output(stdout, &out).map(|()| (out, Vec::new()))
        }
        Some(budget) => write_budgeted(stdout, stderr, input, &[], budget, classify),
    };
    match rendered {
        Ok((rendered_stdout, rendered_stderr)) => {
            record_savings(
                history,
                &ApproximateCounter,
                verb,
                "",
                &[],
                &combine_lossy(input, &[]),
                &combine_lossy(&rendered_stdout, &rendered_stderr),
            );
            0
        }
        Err(WriteOutcome::BrokenPipe) => 0,
        Err(WriteOutcome::Failed(err)) => {
            let _ = writeln!(stderr, "stk: failed to write output: {err}");
            1
        }
        Err(WriteOutcome::AlreadyReported(code)) => code,
    }
}

fn combine_lossy(a: &[u8], b: &[u8]) -> String {
    if b.is_empty() {
        return String::from_utf8_lossy(a).into_owned();
    }
    let mut combined = Vec::with_capacity(a.len() + b.len());
    combined.extend_from_slice(a);
    combined.extend_from_slice(b);
    String::from_utf8_lossy(&combined).into_owned()
}

fn write_budgeted(
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    raw_stdout: &[u8],
    raw_stderr: &[u8],
    budget: usize,
    classify: Classifier,
) -> Result<(Vec<u8>, Vec<u8>), WriteOutcome> {
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
            write_output(stderr, &output.stderr)?;
            Ok((output.stdout, output.stderr))
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
