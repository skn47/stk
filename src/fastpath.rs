use crate::aggregation::repetition;
use crate::normalize::{ansi, boilerplate, long_lines, whitespace};

/// The Janitor + Aggregator fast-path transforms, stopping at the line-sequence stage
/// (before final rendering) so the `Budget`-aware pipeline can chunk and select from the
/// same cleaned-up lines `run` would otherwise just join and print directly.
pub fn lines_from(input: &[u8]) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }

    let stripped = ansi::strip(input);
    let text = String::from_utf8_lossy(&stripped);
    let lines: Vec<String> = text.lines().map(str::to_string).collect();

    // Repetition must run against the true, unmodified line sequence: boilerplate
    // removal or long-line truncation happening first can fabricate false adjacency
    // (removing a divider between two distinct occurrences) or false equality
    // (truncating two different long lines down to the same head/tail).
    let lines = whitespace::normalize(lines);
    let lines = repetition::collapse(lines);
    let lines = boilerplate::filter(lines);
    long_lines::truncate(lines)
}

/// The Janitor + Aggregator fast path: deterministic cleanup, no scoring or budget
/// selection (that's layered on top of this later).
pub fn run(input: &[u8]) -> Vec<u8> {
    let lines = lines_from(input);
    if lines.is_empty() {
        return Vec::new();
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_produces_empty_output() {
        assert_eq!(run(b""), b"");
    }

    #[test]
    fn strips_ansi_then_aggregates_repeats() {
        let input = b"\x1b[31merror\x1b[0m\nretry\nretry\nretry\n";
        assert_eq!(run(input), b"error\nretry [repeated 3x]\n");
    }

    #[test]
    fn full_pipeline_on_a_noisy_log() {
        let input = "\x1b[32mBuilding...\x1b[0m\n----------\nwarning: unused   \n\n\n\ndone\n";
        let output = String::from_utf8(run(input.as_bytes())).unwrap();
        assert_eq!(output, "Building...\nwarning: unused\n\ndone\n");
    }

    #[test]
    fn a_boilerplate_line_between_two_distinct_occurrences_does_not_fake_a_repeat() {
        let input = b"build step A\n----------\nbuild step A\n";
        let output = String::from_utf8(run(input)).unwrap();
        assert_eq!(
            output, "build step A\nbuild step A\n",
            "two genuinely separate occurrences must not be reported as a repeated run \
             just because the divider between them was removed"
        );
    }

    #[test]
    fn a_repeated_separator_run_is_still_removed_as_boilerplate() {
        let input = b"a\n----------\n----------\nb\n";
        let output = String::from_utf8(run(input)).unwrap();
        assert_eq!(output, "a\nb\n");
    }
}
