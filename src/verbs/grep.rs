use std::io::Write;
use std::path::Path;

use crate::budget::tokenizer::ApproximateCounter;
use crate::compression::{write_output, WriteOutcome};
use crate::verbs::file_walk::walk_files;
use crate::verbs::filesystem_budget::{terminated, truncate_sequential, TooSmall};

/// `stk grep <pattern> <path>`: a native, `Budget`-aware search (not an `rg` wrapper).
/// Substring matching only in this release -- no regex engine dependency. Stateless.
pub fn dispatch(
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    budget: Option<usize>,
) -> i32 {
    let mut iter = args.iter();
    let Some(pattern) = iter.next() else {
        let _ = writeln!(stderr, "stk: grep requires a pattern and a path");
        return 2;
    };
    let Some(path) = iter.next() else {
        let _ = writeln!(stderr, "stk: grep requires a path");
        return 2;
    };

    let mut matches = Vec::new();
    if let Err(err) = collect_matches(Path::new(path), pattern, &mut matches) {
        let _ = writeln!(stderr, "stk: failed to search '{path}': {err}");
        return 1;
    }

    let rendered = match budget {
        None => terminated(matches.join("\n")),
        Some(budget) => match truncate_sequential(&matches, budget, &ApproximateCounter) {
            Ok(rendered) => rendered,
            Err(TooSmall { minimum }) => {
                let _ = writeln!(
                    stderr,
                    "stk: --budget {budget} is too small to render any matches (minimum: {minimum})"
                );
                return 2;
            }
        },
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

fn collect_matches(path: &Path, pattern: &str, out: &mut Vec<String>) -> std::io::Result<()> {
    walk_files(path, &mut |file| {
        // A missing/unreadable *root* path is caught by walk_files itself; a file
        // found via directory listing that then fails to read (binary content, a
        // race) is skipped, matching grep's own default binary-file behavior.
        let Ok(content) = std::fs::read_to_string(file) else {
            return;
        };
        for (line_number, line) in content.lines().enumerate() {
            if line.contains(pattern) {
                out.push(format!("{}:{}:{}", file.display(), line_number + 1, line));
            }
        }
    })
}
