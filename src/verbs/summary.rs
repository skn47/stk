use std::io::Write;

use crate::budget::tokenizer::ApproximateCounter;
use crate::capture::executor::CommandExecutor;
use crate::chunk::ChunkKind;
use crate::compression::{write_output, WriteOutcome};
use crate::fastpath;
use crate::history::HistoryStore;
use crate::scoring::relevance;
use crate::verbs::filesystem_budget::render_optionally_budgeted;

/// `stk summary <CMD> [ARGS...]`: runs a command and shows a heuristic condensed
/// summary -- counts plus the first error, not the full output `compile` would give.
pub fn dispatch(
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    executor: &dyn CommandExecutor,
    history: &dyn HistoryStore,
    budget: Option<usize>,
) -> i32 {
    let Some((command, cmd_args)) = args.split_first() else {
        let _ = writeln!(stderr, "stk: summary requires a command");
        return 2;
    };

    let result = match executor.execute(command, cmd_args) {
        Ok(result) => result,
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to run '{command}': {err}");
            return 1;
        }
    };

    let mut lines = fastpath::lines_from(&result.stdout);
    lines.extend(fastpath::lines_from(&result.stderr));

    let mut error_count = 0;
    let mut warning_count = 0;
    let mut first_error = None;
    for line in &lines {
        match relevance::classify(line) {
            (ChunkKind::Error, _) => {
                error_count += 1;
                if first_error.is_none() {
                    first_error = Some(line.clone());
                }
            }
            (ChunkKind::Warning, _) => warning_count += 1,
            _ => {}
        }
    }

    let exit_code = result.exit_info().to_process_exit_code();
    let status = if exit_code == 0 { "ok" } else { "failed" };
    let mut summary_lines = vec![format!(
        "{status}: {error_count} error{}, {warning_count} warning{} ({} lines)",
        if error_count == 1 { "" } else { "s" },
        if warning_count == 1 { "" } else { "s" },
        lines.len(),
    )];
    if let Some(first_error) = first_error {
        summary_lines.push(first_error);
    }

    let rendered = match render_optionally_budgeted(
        &summary_lines,
        budget,
        &ApproximateCounter,
        "a summary",
        stderr,
        history,
        "summary",
        command,
        cmd_args,
        &lines.join("\n"),
    ) {
        Ok(rendered) => rendered,
        Err(code) => return code,
    };

    match write_output(stdout, rendered.as_bytes()) {
        Ok(()) => exit_code,
        Err(WriteOutcome::BrokenPipe) => exit_code,
        Err(WriteOutcome::Failed(err)) => {
            let _ = writeln!(stderr, "stk: failed to write output: {err}");
            1
        }
        Err(WriteOutcome::AlreadyReported(code)) => code,
    }
}
