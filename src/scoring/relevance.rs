use crate::aggregation::repetition;
use crate::chunk::{ChunkKind, Priority};

/// Classifies one line by keyword alone: no command semantics to draw on without a
/// Specialist. An already-collapsed repeated-line marker always classifies as noise,
/// regardless of what it repeats -- that's the whole reason it got collapsed.
pub fn classify(line: &str) -> (ChunkKind, Priority) {
    if repetition::strip_marker(line) != line {
        return (ChunkKind::LogGroup, Priority::P5);
    }
    let lower = line.to_lowercase();
    if lower.contains("error") || lower.contains("panic") || lower.contains("fatal") {
        (ChunkKind::Error, Priority::P0)
    } else if lower.contains("warning") || lower.contains("warn:") {
        (ChunkKind::Warning, Priority::P2)
    } else {
        (ChunkKind::LogGroup, Priority::P3)
    }
}

/// Score is priority-dominated (a full tier always beats a fractional recency bonus from
/// another tier) with a small tiebreak favoring later lines -- terminal output
/// conventionally puts the most decision-relevant content (a final summary, the tail of
/// a long build) near the end.
pub fn score(priority: Priority, position: usize, total: usize) -> f32 {
    let priority_weight = match priority {
        Priority::P0 => 500.0,
        Priority::P1 => 400.0,
        Priority::P2 => 300.0,
        Priority::P3 => 200.0,
        Priority::P4 => 100.0,
        Priority::P5 => 0.0,
    };
    let recency_bonus = if total <= 1 {
        0.0
    } else {
        (position as f32 / (total - 1) as f32) * 50.0
    };
    priority_weight + recency_bonus
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_keyword_classifies_as_p0() {
        assert_eq!(
            classify("error[E0308]: mismatched types"),
            (ChunkKind::Error, Priority::P0)
        );
        assert_eq!(
            classify("panic: index out of bounds"),
            (ChunkKind::Error, Priority::P0)
        );
        assert_eq!(
            classify("fatal: not a git repository"),
            (ChunkKind::Error, Priority::P0)
        );
    }

    #[test]
    fn warning_keyword_classifies_as_p2() {
        assert_eq!(
            classify("warning: unused variable"),
            (ChunkKind::Warning, Priority::P2)
        );
    }

    #[test]
    fn a_collapsed_repeat_marker_classifies_as_p5_noise_even_if_it_mentions_error() {
        assert_eq!(
            classify("error: retry failed [repeated 4x]"),
            (ChunkKind::LogGroup, Priority::P5)
        );
    }

    #[test]
    fn plain_content_classifies_as_p3() {
        assert_eq!(
            classify("Compiling stk v0.1.0"),
            (ChunkKind::LogGroup, Priority::P3)
        );
    }

    #[test]
    fn score_never_lets_recency_override_a_full_priority_tier() {
        let best_p1 = score(Priority::P1, 0, 100);
        let worst_p0 = score(Priority::P0, 0, 100);
        assert!(worst_p0 > best_p1);
    }

    #[test]
    fn score_breaks_ties_within_a_tier_toward_later_lines() {
        let earlier = score(Priority::P3, 0, 10);
        let later = score(Priority::P3, 9, 10);
        assert!(later > earlier);
    }
}
