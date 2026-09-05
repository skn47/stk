/// Trims trailing whitespace per line and collapses runs of blank lines to a single one.
pub fn normalize(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut prev_blank = false;
    for line in lines {
        let trimmed = line.trim_end().to_string();
        let is_blank = trimmed.is_empty();
        if is_blank && prev_blank {
            continue;
        }
        prev_blank = is_blank;
        out.push(trimmed);
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
    fn trims_trailing_whitespace() {
        assert_eq!(
            normalize(lines(&["hello   ", "world\t"])),
            lines(&["hello", "world"])
        );
    }

    #[test]
    fn collapses_runs_of_blank_lines_to_one() {
        assert_eq!(
            normalize(lines(&["a", "", "", "", "b"])),
            lines(&["a", "", "b"])
        );
    }

    #[test]
    fn a_single_blank_line_is_left_alone() {
        assert_eq!(normalize(lines(&["a", "", "b"])), lines(&["a", "", "b"]));
    }
}
