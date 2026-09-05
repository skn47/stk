/// Filters known-boilerplate lines. Generic-pipeline scope only: pure visual separator
/// lines (e.g. `----------`), common across many tools' output regardless of command.
/// Command-specific boilerplate (Cargo/Git build noise) belongs to their own Specialists.
use crate::aggregation::repetition;

const SEPARATOR_CHARS: &[char] = &['-', '=', '*', '_', '~', '#'];
const MIN_SEPARATOR_LEN: usize = 4;

pub fn filter(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| !is_boilerplate(line))
        .collect()
}

fn is_boilerplate(line: &str) -> bool {
    // Runs after repetition::collapse, so a repeated run of separator lines carries a
    // "[repeated Nx]" suffix by now -- check the line's original content, not that suffix.
    is_pure_separator(repetition::strip_marker(line))
}

fn is_pure_separator(line: &str) -> bool {
    let trimmed = line.trim();
    let Some(first) = trimmed.chars().next() else {
        return false;
    };
    trimmed.chars().count() >= MIN_SEPARATOR_LEN
        && SEPARATOR_CHARS.contains(&first)
        && trimmed.chars().all(|c| c == first)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn removes_pure_separator_lines() {
        assert_eq!(
            filter(lines(&["result", "----------", "done"])),
            lines(&["result", "done"])
        );
    }

    #[test]
    fn keeps_content_lines_that_merely_contain_dashes() {
        assert_eq!(
            filter(lines(&["--flag value", "a-b-c"])),
            lines(&["--flag value", "a-b-c"])
        );
    }

    #[test]
    fn a_short_run_of_separator_chars_is_not_treated_as_boilerplate() {
        assert_eq!(filter(lines(&["--", "ok"])), lines(&["--", "ok"]));
    }
}
