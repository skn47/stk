use crate::aggregation::repetition;
use crate::chunk::{ChunkKind, Priority};

const BUILD_PHASE_PREFIXES: &[&str] = &[
    "Compiling",
    "Checking",
    "Finished",
    "Running",
    "Fresh",
    "Downloading",
    "Downloaded",
];

/// Classifies one line of `cargo` output. Line-based, not block-based: an error's
/// surrounding pretty-printer decoration falls through to the generic default.
pub fn classify(line: &str) -> (ChunkKind, Priority) {
    if repetition::strip_marker(line) != line {
        return (ChunkKind::LogGroup, Priority::P5);
    }

    let trimmed = line.trim_start();

    if BUILD_PHASE_PREFIXES
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
    {
        return (ChunkKind::Boilerplate, Priority::P4);
    }

    if trimmed.starts_with("-->") {
        return (ChunkKind::StackFrame, Priority::P0);
    }

    if trimmed.starts_with("error") {
        return (ChunkKind::Error, Priority::P0);
    }

    if trimmed.contains("panicked at") {
        return (ChunkKind::Error, Priority::P0);
    }

    if trimmed.starts_with("left:")
        || trimmed.starts_with("right:")
        || trimmed.contains("assertion")
    {
        return (ChunkKind::Error, Priority::P0);
    }

    // Checked before the broader "contains FAILED" rule below: "test result: FAILED. ..."
    // is a summary line, not an individual failing-test line.
    if trimmed.starts_with("test result:") || trimmed == "failures:" {
        return (ChunkKind::CommandSummary, Priority::P2);
    }

    if trimmed.contains("FAILED") {
        return (ChunkKind::TestFailure, Priority::P0);
    }

    if trimmed.starts_with("warning") {
        return (ChunkKind::Warning, Priority::P2);
    }

    if trimmed.starts_with("test ") && trimmed.ends_with("ok") {
        return (ChunkKind::TestSuccess, Priority::P4);
    }

    (ChunkKind::LogGroup, Priority::P3)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every line below is copied verbatim from a real `cargo check`/`build`/`test` run
    // against a deliberately broken fixture project, not hand-imagined.

    #[test]
    fn compiler_error_code_is_p0() {
        assert_eq!(
            classify("error[E0308]: mismatched types"),
            (ChunkKind::Error, Priority::P0)
        );
    }

    #[test]
    fn error_location_is_p0() {
        assert_eq!(
            classify(" --> src/main.rs:7:30"),
            (ChunkKind::StackFrame, Priority::P0)
        );
    }

    #[test]
    fn final_compile_failure_summary_is_p0() {
        assert_eq!(
            classify(
                "error: could not compile `cargo_fixture_gen` (bin \"cargo_fixture_gen\") due to 1 previous error"
            ),
            (ChunkKind::Error, Priority::P0)
        );
    }

    #[test]
    fn build_phase_noise_is_low_priority() {
        assert_eq!(
            classify("    Checking cargo_fixture_gen v0.1.0 (/some/path)"),
            (ChunkKind::Boilerplate, Priority::P4)
        );
        assert_eq!(
            classify("    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s"),
            (ChunkKind::Boilerplate, Priority::P4)
        );
    }

    #[test]
    fn warning_is_p2() {
        assert_eq!(
            classify("warning: unused variable: `unused_var`"),
            (ChunkKind::Warning, Priority::P2)
        );
    }

    #[test]
    fn warning_location_is_still_p0() {
        assert_eq!(
            classify(" --> src/main.rs:6:9"),
            (ChunkKind::StackFrame, Priority::P0)
        );
    }

    #[test]
    fn failing_test_line_is_p0() {
        assert_eq!(
            classify("test tests::it_adds_correctly ... FAILED"),
            (ChunkKind::TestFailure, Priority::P0)
        );
    }

    #[test]
    fn passing_test_line_is_low_priority() {
        assert_eq!(
            classify("test tests::it_passes ... ok"),
            (ChunkKind::TestSuccess, Priority::P4)
        );
    }

    #[test]
    fn panic_location_is_p0() {
        assert_eq!(
            classify("thread 'tests::it_adds_correctly' (919297) panicked at src/main.rs:15:9:"),
            (ChunkKind::Error, Priority::P0)
        );
    }

    #[test]
    fn assertion_detail_lines_are_p0() {
        assert_eq!(
            classify("assertion `left == right` failed"),
            (ChunkKind::Error, Priority::P0)
        );
        assert_eq!(classify("  left: 4"), (ChunkKind::Error, Priority::P0));
        assert_eq!(classify(" right: 5"), (ChunkKind::Error, Priority::P0));
    }

    #[test]
    fn test_result_summary_is_p2() {
        assert_eq!(
            classify(
                "test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s"
            ),
            (ChunkKind::CommandSummary, Priority::P2)
        );
    }

    #[test]
    fn unrelated_content_falls_back_to_generic_default() {
        assert_eq!(
            classify(
                "  = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default"
            ),
            (ChunkKind::LogGroup, Priority::P3)
        );
    }

    #[test]
    fn a_collapsed_repeat_marker_is_still_p5_noise() {
        assert_eq!(
            classify("Compiling foo v0.1.0 [repeated 20x]"),
            (ChunkKind::LogGroup, Priority::P5)
        );
    }
}
