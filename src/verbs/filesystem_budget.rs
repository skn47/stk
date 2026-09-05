use std::io::Write;

use crate::budget::tokenizer::TokenCounter;

#[derive(Debug)]
pub struct TooSmall {
    pub minimum: usize,
}

/// The largest `k` in `0..=max_k` for which `fits(k)` holds, assuming `fits` is
/// monotonic (true for small `k`, false past some point) -- the shared core of both
/// head+tail truncation strategies below, which differ only in what a "unit" is
/// (chars vs. lines) and how the kept portions get joined back together.
fn largest_k_that_fits(max_k: usize, fits: impl Fn(usize) -> bool) -> usize {
    let mut lo = 0usize;
    let mut hi = max_k;
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

/// Applies `Budget` truncation if given (else joins everything), writing a clear error
/// to `stderr` and returning the exit code to use if the budget is below the floor.
/// Shared by every verb whose content is a flat, sequentially-truncatable line list
/// (`grep`/`find`/`diff`/`err`/`summary`).
pub fn render_optionally_budgeted(
    items: &[String],
    budget: Option<usize>,
    counter: &dyn TokenCounter,
    what: &str,
    stderr: &mut dyn Write,
) -> Result<String, i32> {
    match budget {
        None => Ok(terminated(items.join("\n"))),
        Some(budget) => match truncate_sequential(items, budget, counter) {
            Ok(rendered) => Ok(rendered),
            Err(TooSmall { minimum }) => {
                let _ = writeln!(
                    stderr,
                    "stk: --budget {budget} is too small to render {what} (minimum: {minimum})"
                );
                Err(2)
            }
        },
    }
}

fn omission_marker(count: usize) -> String {
    if count == 1 {
        "[stk: omitted 1 line]".to_string()
    } else {
        format!("[stk: omitted {count} lines]")
    }
}

/// Ensures a trailing newline (matching `cat`/`grep`/`find`'s own convention) without
/// adding one to genuinely empty output (e.g. zero matches).
pub fn terminated(mut s: String) -> String {
    if !s.is_empty() && !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

fn omission_marker_chars(count: usize) -> String {
    if count == 1 {
        "[stk: omitted 1 char]".to_string()
    } else {
        format!("[stk: omitted {count} chars]")
    }
}

/// Character-based head+tail truncation for a single long blob with no natural line
/// breaks (e.g. compacted JSON) -- `truncate_head_tail` below operates on lines, which
/// degrades to "keep nothing" for content that's just one unbroken line.
pub fn truncate_chars_head_tail(
    content: &str,
    budget: usize,
    counter: &dyn TokenCounter,
) -> Result<String, TooSmall> {
    if counter.count(content) <= budget {
        return Ok(content.to_string());
    }
    let chars: Vec<char> = content.chars().collect();
    let marker_reserve = counter.count(&omission_marker_chars(chars.len()));
    if budget < marker_reserve {
        return Err(TooSmall {
            minimum: marker_reserve,
        });
    }
    let remaining = budget - marker_reserve;

    let fits = |k: usize| -> bool {
        let head: String = chars[..k].iter().collect();
        let tail: String = chars[chars.len() - k..].iter().collect();
        counter.count(&head) + counter.count(&tail) <= remaining
    };

    let k = largest_k_that_fits(chars.len() / 2, fits);
    let omitted = chars.len() - 2 * k;
    let head: String = chars[..k].iter().collect();
    let tail: String = chars[chars.len() - k..].iter().collect();
    Ok(format!("{head}{}{tail}", omission_marker_chars(omitted)))
}

/// Conservative text-based slicing for a single file's content: keeps the head and tail
/// lines, dropping the middle, rather than any structural (Tree-sitter) understanding.
pub fn truncate_head_tail(
    content: &str,
    budget: usize,
    counter: &dyn TokenCounter,
) -> Result<String, TooSmall> {
    if counter.count(content) <= budget {
        return Ok(terminated(content.to_string()));
    }
    let lines: Vec<&str> = content.lines().collect();
    let marker_reserve = counter.count(&omission_marker(lines.len()));
    if budget < marker_reserve {
        return Err(TooSmall {
            minimum: marker_reserve,
        });
    }
    let remaining = budget - marker_reserve;

    let fits = |k: usize| -> bool {
        let head: String = lines[..k].join("\n");
        let tail: String = lines[lines.len() - k..].join("\n");
        counter.count(&head) + counter.count(&tail) <= remaining
    };

    let k = largest_k_that_fits(lines.len() / 2, fits);
    let omitted = lines.len() - 2 * k;
    let head = lines[..k].join("\n");
    let tail = lines[lines.len() - k..].join("\n");
    Ok(terminated(format!(
        "{head}\n{}\n{tail}",
        omission_marker(omitted)
    )))
}

/// Keeps items from the start until the budget runs out, then a single omission marker
/// for the rest -- suited to a flat, order-independent-importance list (grep matches,
/// find results), unlike a single file's head/tail-preserving shape above.
pub fn truncate_sequential(
    items: &[String],
    budget: usize,
    counter: &dyn TokenCounter,
) -> Result<String, TooSmall> {
    let joined = items.join("\n");
    if counter.count(&joined) <= budget {
        return Ok(terminated(joined));
    }
    let marker_reserve = counter.count(&omission_marker(items.len()));
    if budget < marker_reserve {
        return Err(TooSmall {
            minimum: marker_reserve,
        });
    }
    let mut remaining = budget - marker_reserve;
    let mut kept = 0;
    for item in items {
        let cost = counter.count(item);
        if cost > remaining {
            break;
        }
        remaining -= cost;
        kept += 1;
    }

    let omitted = items.len() - kept;
    if omitted == 0 {
        Ok(terminated(items.join("\n")))
    } else {
        Ok(terminated(format!(
            "{}\n{}",
            items[..kept].join("\n"),
            omission_marker(omitted)
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::tokenizer::ApproximateCounter;

    fn lines(n: usize) -> String {
        (0..n)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn content_that_already_fits_is_untouched() {
        let content = "short file\n";
        assert_eq!(
            truncate_head_tail(content, 100, &ApproximateCounter).unwrap(),
            content
        );
    }

    #[test]
    fn a_too_large_file_is_truncated_keeping_head_and_tail() {
        let content = lines(200);
        let result = truncate_head_tail(&content, 60, &ApproximateCounter).unwrap();

        assert!(result.starts_with("line 0"));
        assert!(result.trim_end().ends_with("line 199"));
        assert!(result.contains("[stk: omitted"));
        assert!(ApproximateCounter.count(&result) <= 60);
    }

    #[test]
    fn a_budget_below_the_marker_floor_is_refused() {
        let content = lines(200);
        let result = truncate_head_tail(&content, 1, &ApproximateCounter);
        assert!(result.is_err());
    }

    #[test]
    fn chars_head_tail_keeps_both_ends_of_an_unbroken_blob() {
        let blob = format!("{{\"start\":true,{}\"end\":true}}", "x".repeat(2000));
        let result = truncate_chars_head_tail(&blob, 60, &ApproximateCounter).unwrap();

        assert!(result.starts_with("{\"start\":true,"));
        assert!(result.ends_with("\"end\":true}"));
        assert!(result.contains("[stk: omitted"));
        assert!(ApproximateCounter.count(&result) <= 60);
    }

    #[test]
    fn sequential_content_that_already_fits_is_untouched_no_spurious_marker() {
        // Regression: the budget check must run against what actually needs rendering,
        // not unconditionally subtract the marker reserve before checking whether the
        // full content even needs truncating at all.
        let items: Vec<String> = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let joined_tokens = ApproximateCounter.count(&items.join("\n"));
        let result = truncate_sequential(&items, joined_tokens, &ApproximateCounter).unwrap();

        assert_eq!(result, "a\nb\nc\n");
        assert!(!result.contains("[stk: omitted"));
    }

    #[test]
    fn sequential_keeps_a_prefix_and_marks_the_rest_omitted() {
        let items: Vec<String> = (0..50).map(|i| format!("match {i}")).collect();
        let result = truncate_sequential(&items, 30, &ApproximateCounter).unwrap();

        assert!(result.starts_with("match 0"));
        assert!(result.contains("[stk: omitted"));
        assert!(!result.contains("match 49"));
    }

    #[test]
    fn sequential_with_no_truncation_needed_has_no_marker() {
        let items: Vec<String> = vec!["a".to_string(), "b".to_string()];
        let result = truncate_sequential(&items, 500, &ApproximateCounter).unwrap();
        assert_eq!(result, "a\nb\n");
    }
}
