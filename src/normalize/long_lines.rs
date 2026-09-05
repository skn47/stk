/// Keeps a single pathological line (e.g. a minified blob) from defeating the rest of
/// compression on its own: past this length, keep the head and tail and mark the rest.
const MAX_LINE_CHARS: usize = 2000;
const KEEP_HEAD_CHARS: usize = 200;
const KEEP_TAIL_CHARS: usize = 200;

pub fn truncate(lines: Vec<String>) -> Vec<String> {
    lines.into_iter().map(|line| truncate_line(&line)).collect()
}

fn truncate_line(line: &str) -> String {
    // Byte length is always >= char count, so this cheap check alone already rules out
    // truncation for the overwhelming majority of (short, ASCII-ish) lines without
    // needing to collect into a Vec<char> at all.
    if line.len() <= MAX_LINE_CHARS {
        return line.to_string();
    }
    let chars: Vec<char> = line.chars().collect();
    if chars.len() <= MAX_LINE_CHARS {
        return line.to_string();
    }
    let head: String = chars[..KEEP_HEAD_CHARS].iter().collect();
    let tail: String = chars[chars.len() - KEEP_TAIL_CHARS..].iter().collect();
    let omitted = chars.len() - KEEP_HEAD_CHARS - KEEP_TAIL_CHARS;
    format!("{head} ... [stk: line truncated, {omitted} chars omitted] ... {tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_short_lines_untouched() {
        let lines = vec!["a short line".to_string()];
        assert_eq!(truncate(lines.clone()), lines);
    }

    #[test]
    fn truncates_a_pathologically_long_line_keeping_head_and_tail() {
        let long_line = format!("HEAD_MARKER{}TAIL_MARKER", "x".repeat(3000));
        let result = truncate(vec![long_line]);

        assert_eq!(result.len(), 1);
        assert!(result[0].starts_with("HEAD_MARKER"));
        assert!(result[0].ends_with("TAIL_MARKER"));
        assert!(result[0].contains("chars omitted"));
        assert!(result[0].len() < 3000);
    }
}
