use std::io::Write;

use crate::budget::tokenizer::ApproximateCounter;
use crate::capture::executor::CommandExecutor;
use crate::chunk::ChunkKind;
use crate::compression::{write_output, WriteOutcome};
use crate::fastpath;
use crate::scoring::relevance;
use crate::verbs::filesystem_budget::render_optionally_budgeted;

/// `stk err <CMD> [ARGS...]`: runs a command and shows only its errors/warnings -- a
/// hard filter (matching real `rtk err`), not `compile`'s soft priority-based packing.
pub fn dispatch(
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    executor: &dyn CommandExecutor,
    budget: Option<usize>,
) -> i32 {
    let Some((command, cmd_args)) = args.split_first() else {
        let _ = writeln!(stderr, "stk: err requires a command");
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
    let filtered: Vec<String> = lines
        .into_iter()
        .filter(|line| {
            matches!(
                relevance::classify(line).0,
                ChunkKind::Error | ChunkKind::Warning
            )
        })
        .collect();

    let rendered = match render_optionally_budgeted(
        &filtered,
        budget,
        &ApproximateCounter,
        "any errors/warnings",
        stderr,
    ) {
        Ok(rendered) => rendered,
        Err(code) => return code,
    };

    match write_output(stdout, rendered.as_bytes()) {
        Ok(()) => result.exit_info().to_process_exit_code(),
        Err(WriteOutcome::BrokenPipe) => result.exit_info().to_process_exit_code(),
        Err(WriteOutcome::Failed(err)) => {
            let _ = writeln!(stderr, "stk: failed to write output: {err}");
            1
        }
        Err(WriteOutcome::AlreadyReported(code)) => code,
    }
}
