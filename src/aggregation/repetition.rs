/// If `line` ends with the `[repeated Nx]` marker [`collapse`] appends, returns the line
/// without it; otherwise returns `line` unchanged. Lets later stages (e.g. boilerplate
/// filtering) recognize a line's original content after collapsing has run.
pub fn strip_marker(line: &str) -> &str {
    const PREFIX: &str = " [repeated ";
    let Some(idx) = line.rfind(PREFIX) else {
        return line;
    };
    let Some(digits) = line[idx + PREFIX.len()..].strip_suffix("x]") else {
        return line;
    };
    if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
        &line[..idx]
    } else {
        line
    }
}

/// Collapses runs of exactly-repeated consecutive lines into `<line> [repeated Nx]`.
pub fn collapse(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let mut j = i + 1;
        while j < lines.len() && lines[j] == lines[i] {
            j += 1;
        }
        let count = j - i;
        if count >= 2 {
            out.push(format!("{} [repeated {count}x]", lines[i]));
        } else {
            out.push(lines[i].clone());
        }
        i = j;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn collapses_a_run_of_identical_consecutive_lines() {
        assert_eq!(
            collapse(lines(&[
                "retrying connection...",
                "retrying connection...",
                "retrying connection...",
                "retrying connection..."
            ])),
            lines(&["retrying connection... [repeated 4x]"])
        );
    }

    #[test]
    fn a_single_occurrence_is_left_unmodified() {
        assert_eq!(collapse(lines(&["hello"])), lines(&["hello"]));
    }

    #[test]
    fn non_consecutive_repeats_are_not_collapsed() {
        assert_eq!(collapse(lines(&["a", "b", "a"])), lines(&["a", "b", "a"]));
    }

    #[test]
    fn different_runs_are_collapsed_independently() {
        assert_eq!(
            collapse(lines(&["a", "a", "b", "b", "b"])),
            lines(&["a [repeated 2x]", "b [repeated 3x]"])
        );
    }

    #[test]
    fn strip_marker_recovers_the_original_line() {
        assert_eq!(strip_marker("hello [repeated 4x]"), "hello");
    }

    #[test]
    fn strip_marker_leaves_an_unmarked_line_alone() {
        assert_eq!(strip_marker("hello"), "hello");
        assert_eq!(
            strip_marker("literally [repeated in prose]"),
            "literally [repeated in prose]"
        );
    }
}
