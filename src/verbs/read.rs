use std::borrow::Cow;
use std::io::Write;

use crate::budget::tokenizer::ApproximateCounter;
use crate::compression::{write_output, WriteOutcome};
use crate::history::{record_savings, HistoryStore};
use crate::verbs::filesystem_budget::{truncate_head_tail, TooSmall};

/// `stk read <file>`: a native, `Budget`-aware file reader (not a `cat` wrapper).
/// Stateless -- unaffected by anything but the file's current content.
pub fn dispatch(
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    history: &dyn HistoryStore,
    budget: Option<usize>,
) -> i32 {
    let Some(path) = args.first() else {
        let _ = writeln!(stderr, "stk: read requires a file path");
        return 2;
    };

    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to read '{path}': {err}");
            return 1;
        }
    };

    let rendered: Cow<str> = match budget {
        None => Cow::Borrowed(&content),
        Some(budget) => match truncate_head_tail(&content, budget, &ApproximateCounter) {
            Ok(rendered) => Cow::Owned(rendered),
            Err(TooSmall { minimum }) => {
                let _ = writeln!(
                    stderr,
                    "stk: --budget {budget} is too small to render this file (minimum: {minimum})"
                );
                return 2;
            }
        },
    };

    record_savings(
        history,
        &ApproximateCounter,
        "read",
        "",
        args,
        &content,
        &rendered,
    );

    match write_output(stdout, rendered.as_bytes()) {
        Ok(()) | Err(WriteOutcome::BrokenPipe) => 0,
        Err(WriteOutcome::Failed(err)) => {
            let _ = writeln!(stderr, "stk: failed to write output: {err}");
            1
        }
        Err(WriteOutcome::AlreadyReported(code)) => code,
    }
}
