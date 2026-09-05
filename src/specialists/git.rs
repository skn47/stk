use crate::aggregation::repetition;
use crate::chunk::{ChunkKind, Priority};

/// Classifies one line of `git` output (`status`/`diff`), verified against real
/// invocations against deliberately modified/renamed/conflicted repos.
pub fn classify(line: &str) -> (ChunkKind, Priority) {
    if repetition::strip_marker(line) != line {
        return (ChunkKind::LogGroup, Priority::P5);
    }

    let trimmed = line.trim_start();

    // Rename info: kept P0 rather than falling through to an unrelated delete+add.
    if trimmed.starts_with("similarity index")
        || trimmed.starts_with("rename from")
        || trimmed.starts_with("rename to")
    {
        return (ChunkKind::DiffHunk, Priority::P0);
    }

    // The file-association anchor for whatever hunks follow -- kept P0 so a hunk never
    // survives budget selection while losing which file it belongs to.
    if trimmed.starts_with("diff --git") {
        return (ChunkKind::DiffHunk, Priority::P0);
    }

    if trimmed.starts_with("index ") {
        return (ChunkKind::Boilerplate, Priority::P4);
    }

    if trimmed.starts_with("new file mode")
        || trimmed.starts_with("deleted file mode")
        || trimmed.starts_with("old mode")
        || trimmed.starts_with("new mode")
    {
        return (ChunkKind::DiffHunk, Priority::P1);
    }

    if trimmed.starts_with("--- ") || trimmed.starts_with("+++ ") {
        return (ChunkKind::DiffHunk, Priority::P1);
    }

    if trimmed.starts_with("@@") {
        return (ChunkKind::DiffHunk, Priority::P0);
    }

    // Merge-conflict markers: a real conflict, not boilerplate.
    if trimmed.starts_with("<<<<<<<")
        || trimmed.starts_with("=======")
        || trimmed.starts_with(">>>>>>>")
    {
        return (ChunkKind::Error, Priority::P0);
    }

    // Branch-divergence info ("Your branch is ahead of ...", "... have diverged, ...").
    if trimmed.starts_with("Your branch") {
        return (ChunkKind::CommandSummary, Priority::P0);
    }

    // Human-readable status entries (`git status`, non-porcelain).
    if trimmed.starts_with("modified:")
        || trimmed.starts_with("new file:")
        || trimmed.starts_with("renamed:")
        || trimmed.starts_with("deleted:")
        || trimmed.starts_with("both modified:")
        || trimmed.starts_with("both added:")
        || trimmed.starts_with("both deleted:")
        || trimmed.starts_with("added by")
        || trimmed.starts_with("deleted by")
        || trimmed.starts_with("copied:")
    {
        return (ChunkKind::DiffHunk, Priority::P0);
    }

    if trimmed == "Unmerged paths:" || trimmed == "You have unmerged paths." {
        return (ChunkKind::CommandSummary, Priority::P2);
    }

    // Porcelain status (`git status --porcelain`, and what rtk's own compact status
    // uses): a two-character status code, a space, then a path (or "old -> new"). Checked
    // on the *untrimmed* line -- an unstaged-only change's leading space (e.g. " M
    // foo.rs") is the first status column, not incidental whitespace to trim away.
    if let Some(kind_and_priority) = classify_porcelain_status_line(line) {
        return kind_and_priority;
    }

    // Diff content lines: the leading marker must be checked on the *untrimmed* line --
    // trimming would eat the single leading space that marks an unchanged context line,
    // making it indistinguishable from anything else that starts with whitespace.
    if line.starts_with('+') || line.starts_with('-') {
        return (ChunkKind::DiffHunk, Priority::P1);
    }

    (ChunkKind::LogGroup, Priority::P3)
}

const PORCELAIN_STATUS_CHARS: &str = "MADRCU?";

fn classify_porcelain_status_line(trimmed: &str) -> Option<(ChunkKind, Priority)> {
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    let second = chars.next()?;
    let third = chars.next()?;
    if third != ' ' {
        return None;
    }
    let is_status_char = |c: char| c == ' ' || PORCELAIN_STATUS_CHARS.contains(c);
    if !is_status_char(first) || !is_status_char(second) || (first == ' ' && second == ' ') {
        return None;
    }
    if first == '?' && second == '?' {
        return Some((ChunkKind::LogGroup, Priority::P2));
    }
    Some((ChunkKind::DiffHunk, Priority::P0))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every line below is copied verbatim from real `git status`/`git diff` output
    // against a deliberately modified/renamed/conflicted repo, not hand-imagined.

    #[test]
    fn porcelain_modified_and_added_and_renamed_are_p0() {
        assert_eq!(classify("MM foo.rs"), (ChunkKind::DiffHunk, Priority::P0));
        assert_eq!(
            classify("A  new_file.rs"),
            (ChunkKind::DiffHunk, Priority::P0)
        );
        assert_eq!(
            classify("R  old_name.rs -> renamed_name.rs"),
            (ChunkKind::DiffHunk, Priority::P0)
        );
        assert_eq!(classify("UU foo.rs"), (ChunkKind::DiffHunk, Priority::P0));
    }

    #[test]
    fn porcelain_untracked_is_lower_priority() {
        assert_eq!(
            classify("?? untracked_only.rs"),
            (ChunkKind::LogGroup, Priority::P2)
        );
    }

    #[test]
    fn porcelain_unstaged_only_change_keeps_its_leading_space_column() {
        // "M foo.rs" alone (post-trim) would misread as a one-character status; the
        // leading space here is real porcelain-format data, not incidental whitespace.
        assert_eq!(classify(" M foo.rs"), (ChunkKind::DiffHunk, Priority::P0));
    }

    #[test]
    fn human_readable_status_entries_are_p0() {
        assert_eq!(
            classify("\tmodified:   foo.rs"),
            (ChunkKind::DiffHunk, Priority::P0)
        );
        assert_eq!(
            classify("\trenamed:    old_name.rs -> renamed_name.rs"),
            (ChunkKind::DiffHunk, Priority::P0)
        );
        assert_eq!(
            classify("\tboth modified:   foo.rs"),
            (ChunkKind::DiffHunk, Priority::P0)
        );
        assert_eq!(
            classify("\tboth deleted:    foo.rs"),
            (ChunkKind::DiffHunk, Priority::P0)
        );
    }

    #[test]
    fn diff_git_header_is_p0() {
        assert_eq!(
            classify("diff --git a/foo.rs b/foo.rs"),
            (ChunkKind::DiffHunk, Priority::P0)
        );
    }

    #[test]
    fn rename_info_is_p0_not_delete_and_add() {
        assert_eq!(
            classify("similarity index 100%"),
            (ChunkKind::DiffHunk, Priority::P0)
        );
        assert_eq!(
            classify("rename from old_name.rs"),
            (ChunkKind::DiffHunk, Priority::P0)
        );
        assert_eq!(
            classify("rename to renamed_name.rs"),
            (ChunkKind::DiffHunk, Priority::P0)
        );
    }

    #[test]
    fn index_metadata_line_is_low_priority() {
        assert_eq!(
            classify("index 70b9567..087a992 100644"),
            (ChunkKind::Boilerplate, Priority::P4)
        );
    }

    #[test]
    fn hunk_header_is_p0() {
        assert_eq!(
            classify("@@ -1,10 +1,11 @@ fn foo() {"),
            (ChunkKind::DiffHunk, Priority::P0)
        );
    }

    #[test]
    fn added_and_removed_diff_lines_are_p1() {
        assert_eq!(
            classify("-    let a = 1;"),
            (ChunkKind::DiffHunk, Priority::P1)
        );
        assert_eq!(
            classify("+    let a = 100;"),
            (ChunkKind::DiffHunk, Priority::P1)
        );
    }

    #[test]
    fn conflict_markers_are_p0() {
        assert_eq!(classify("<<<<<<< HEAD"), (ChunkKind::Error, Priority::P0));
        assert_eq!(classify("======="), (ChunkKind::Error, Priority::P0));
        assert_eq!(
            classify(">>>>>>> feature-branch"),
            (ChunkKind::Error, Priority::P0)
        );
    }

    #[test]
    fn branch_divergence_is_p0() {
        assert_eq!(
            classify("Your branch is ahead of 'origin/main' by 2 commits."),
            (ChunkKind::CommandSummary, Priority::P0)
        );
        assert_eq!(
            classify(
                "Your branch and 'origin/main' have diverged, and have 3 and 2 different commits each, respectively."
            ),
            (ChunkKind::CommandSummary, Priority::P0)
        );
    }

    #[test]
    fn unmerged_paths_header_is_p2() {
        assert_eq!(
            classify("Unmerged paths:"),
            (ChunkKind::CommandSummary, Priority::P2)
        );
    }

    #[test]
    fn hint_and_prose_lines_fall_back_to_generic_default() {
        assert_eq!(
            classify("  (use \"git restore --staged <file>...\" to unstage)"),
            (ChunkKind::LogGroup, Priority::P3)
        );
        assert_eq!(
            classify("On branch master"),
            (ChunkKind::LogGroup, Priority::P3)
        );
    }

    #[test]
    fn a_collapsed_repeat_marker_is_still_p5_noise() {
        assert_eq!(
            classify("M path/to/file.rs [repeated 20x]"),
            (ChunkKind::LogGroup, Priority::P5)
        );
    }
}
