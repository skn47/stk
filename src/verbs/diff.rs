use std::io::{Read, Write};

use crate::budget::tokenizer::ApproximateCounter;
use crate::compression::{write_output, WriteOutcome};
use crate::history::HistoryStore;
use crate::verbs::filesystem_budget::render_optionally_budgeted;

/// `stk diff [FILE|-]`: condenses existing unified-diff text down to hunk headers and
/// changed lines. Not a two-file diff generator -- the input is already diff-formatted.
pub fn dispatch(
    args: &[String],
    mut stdin: impl Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    history: &dyn HistoryStore,
    budget: Option<usize>,
) -> i32 {
    let content = match args.first() {
        Some(path) if path != "-" => match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                let _ = writeln!(stderr, "stk: failed to read '{path}': {err}");
                return 1;
            }
        },
        _ => {
            let mut buf = String::new();
            if let Err(err) = stdin.read_to_string(&mut buf) {
                let _ = writeln!(stderr, "stk: failed to read stdin: {err}");
                return 1;
            }
            buf
        }
    };

    let condensed = condense(&content);

    let rendered = match render_optionally_budgeted(
        &condensed,
        budget,
        &ApproximateCounter,
        "any diff content",
        stderr,
        history,
        "diff",
        "",
        args,
        &content,
    ) {
        Ok(rendered) => rendered,
        Err(code) => return code,
    };

    match write_output(stdout, rendered.as_bytes()) {
        Ok(()) | Err(WriteOutcome::BrokenPipe) => 0,
        Err(WriteOutcome::Failed(err)) => {
            let _ = writeln!(stderr, "stk: failed to write output: {err}");
            1
        }
        Err(WriteOutcome::AlreadyReported(code)) => code,
    }
}

/// Tracks whether we're still in a file's header preamble (before its first `@@`):
/// only there do "---"/"+++ "-prefixed lines mean file-path headers rather than real
/// changed content that happens to start the same way (e.g. a removed line whose own
/// text was "-- note", rendering as "--- note" once the diff marker is prepended).
fn condense(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut past_file_header = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("diff --git") {
            out.push(line.to_string());
            past_file_header = false;
        } else if trimmed.starts_with("@@") {
            out.push(line.to_string());
            past_file_header = true;
        } else if !past_file_header {
            if trimmed.starts_with("similarity index")
                || trimmed.starts_with("rename from")
                || trimmed.starts_with("rename to")
            {
                out.push(line.to_string());
            }
            // else: index/---/+++/other preamble noise, unambiguously dropped since
            // we know we're still in the header region regardless of the line's text.
        } else if line.starts_with('+') || line.starts_with('-') {
            out.push(line.to_string());
        }
    }
    out
}
