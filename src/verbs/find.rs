use std::io::Write;
use std::path::Path;

use crate::budget::tokenizer::ApproximateCounter;
use crate::compression::{write_output, WriteOutcome};
use crate::verbs::file_walk::walk_files;
use crate::verbs::filesystem_budget::{terminated, truncate_sequential, TooSmall};

/// `stk find <path> [--name PATTERN]`: a native, `Budget`-aware recursive file listing
/// (not a `find` wrapper). `--name` is a substring match, not a glob. Stateless.
pub fn dispatch(
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    budget: Option<usize>,
) -> i32 {
    let mut path = None;
    let mut name_filter = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--name" {
            let Some(value) = iter.next() else {
                let _ = writeln!(stderr, "stk: find --name requires a value");
                return 2;
            };
            name_filter = Some(value.as_str());
        } else if path.is_none() {
            path = Some(arg.as_str());
        }
    }
    let path = path.unwrap_or(".");

    let mut entries = Vec::new();
    if let Err(err) = collect_entries(Path::new(path), name_filter, &mut entries) {
        let _ = writeln!(stderr, "stk: failed to search '{path}': {err}");
        return 1;
    }
    entries.sort();

    let rendered = match budget {
        None => terminated(entries.join("\n")),
        Some(budget) => match truncate_sequential(&entries, budget, &ApproximateCounter) {
            Ok(rendered) => rendered,
            Err(TooSmall { minimum }) => {
                let _ = writeln!(
                    stderr,
                    "stk: --budget {budget} is too small to render any results (minimum: {minimum})"
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

fn collect_entries(
    path: &Path,
    name_filter: Option<&str>,
    out: &mut Vec<String>,
) -> std::io::Result<()> {
    walk_files(path, &mut |file| {
        let name_matches = match name_filter {
            None => true,
            Some(filter) => file
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains(filter))
                .unwrap_or(false),
        };
        if name_matches {
            out.push(file.display().to_string());
        }
    })
}
