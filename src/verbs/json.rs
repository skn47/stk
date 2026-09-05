use std::io::Write;

use serde_json::Value;

use crate::budget::tokenizer::ApproximateCounter;
use crate::compression::{write_output, WriteOutcome};
use crate::verbs::filesystem_budget::{truncate_chars_head_tail, TooSmall};

/// `stk json <FILE> [--keys-only]`: compacts a JSON file (minified by default, or
/// structure-only with `--keys-only`, matching real `rtk json`'s flag).
pub fn dispatch(
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    budget: Option<usize>,
) -> i32 {
    let mut path = None;
    let mut keys_only = false;
    for arg in args {
        if arg == "--keys-only" {
            keys_only = true;
        } else if path.is_none() {
            path = Some(arg.as_str());
        }
    }
    let Some(path) = path else {
        let _ = writeln!(stderr, "stk: json requires a file path");
        return 2;
    };

    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to read '{path}': {err}");
            return 1;
        }
    };
    let value: Value = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(err) => {
            let _ = writeln!(stderr, "stk: '{path}' is not valid JSON: {err}");
            return 1;
        }
    };
    let value = if keys_only {
        strip_values(&value)
    } else {
        value
    };
    let compacted = match serde_json::to_string(&value) {
        Ok(compacted) => compacted,
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to serialize '{path}': {err}");
            return 1;
        }
    };

    let rendered = match budget {
        None => compacted,
        Some(budget) => match truncate_chars_head_tail(&compacted, budget, &ApproximateCounter) {
            Ok(rendered) => rendered,
            Err(TooSmall { minimum }) => {
                let _ = writeln!(
                    stderr,
                    "stk: --budget {budget} is too small to render this JSON (minimum: {minimum})"
                );
                return 2;
            }
        },
    };

    match write_output(stdout, format!("{rendered}\n").as_bytes()) {
        Ok(()) | Err(WriteOutcome::BrokenPipe) => 0,
        Err(WriteOutcome::Failed(err)) => {
            let _ = writeln!(stderr, "stk: failed to write output: {err}");
            1
        }
        Err(WriteOutcome::AlreadyReported(code)) => code,
    }
}

/// Replaces every leaf value with `null`, keeping the object/array structure -- lets
/// you see a large JSON's shape without its bulk data.
fn strip_values(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), strip_values(v)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(strip_values).collect()),
        _ => Value::Null,
    }
}
