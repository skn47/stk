/// Identifies a `Chunk` within one compile invocation. Not stable across invocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkId(pub usize);

/// Classifies one line into a `ChunkKind`/`Priority` pair. The generic pipeline and each
/// Specialist each supply their own, letting the same `Budget` selection code work for all.
pub type Classifier = fn(&str) -> (ChunkKind, Priority);

/// P0 is mandatory; P1-P5 are optional, packed into the budget by score. Only
/// P0/P2/P3/P5 are reachable from the generic (no-Specialist) classifier for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
    P4,
    P5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkKind {
    Error,
    Warning,
    StackFrame,
    SourceFunction,
    SourceClass,
    SourceWindow,
    TestFailure,
    TestSuccess,
    DiffHunk,
    LogGroup,
    CommandSummary,
    Documentation,
    Boilerplate,
    /// A real `Chunk`, not a separate render-time concept -- see ADR-0001. Selected,
    /// scored, and counted through the exact same path as everything else.
    OmissionMarker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
pub struct SourceMetadata {
    pub stream: Stream,
    /// 0-indexed position of this chunk's line within its stream, after ticket 05's
    /// fast-path transforms -- used to render selected chunks back in original order.
    pub line_number: usize,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: ChunkId,
    pub kind: ChunkKind,
    pub text: String,
    pub source: SourceMetadata,
    pub token_count: usize,
    pub score: f32,
    pub priority: Priority,
    /// Unused until dependency-aware selection lands (see the design doc's §20
    /// "later improvements"); always empty for now.
    pub relationships: Vec<ChunkId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_ordering_places_p0_first() {
        let mut priorities = vec![Priority::P3, Priority::P0, Priority::P5, Priority::P1];
        priorities.sort();
        assert_eq!(
            priorities,
            vec![Priority::P0, Priority::P1, Priority::P3, Priority::P5]
        );
    }
}
