use std::collections::HashSet;

use crate::chunk::{Chunk, ChunkId, ChunkKind, Priority, SourceMetadata, Stream};
use crate::scoring::relevance;

use super::tokenizer::TokenCounter;

const P0_SHRINK_MARKER_TEXT: &str = "[stk: P0 content truncated to fit budget]";
const STREAMS: [Stream; 2] = [Stream::Stdout, Stream::Stderr];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetError {
    /// `--budget` was below `marker_reserve + min_p0_floor`: too small to honor even in
    /// principle, refused rather than rendered degenerately.
    TooSmall { budget: usize, minimum: usize },
    /// The render-and-recount pass found the actual rendered output still over budget
    /// after dropping every optional chunk -- an internal-consistency bug (the render
    /// pass expanded content beyond its chunk-level estimate), not a user-facing case.
    Internal,
}

#[derive(Debug)]
pub struct BudgetedOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// The Budget Selection Algorithm: always includes P0 chunks (shrinking them if they
/// alone exceed budget), packs optional chunks by score into what remains, and renders
/// with omission-marker `Chunk`s (ADR-0001) for whatever didn't fit.
pub fn select(
    stdout_lines: &[String],
    stderr_lines: &[String],
    budget: usize,
    counter: &dyn TokenCounter,
) -> Result<BudgetedOutput, BudgetError> {
    let mut next_id = 0usize;
    let mut chunks = build_chunks(stdout_lines, Stream::Stdout, &mut next_id, counter);
    chunks.extend(build_chunks(
        stderr_lines,
        Stream::Stderr,
        &mut next_id,
        counter,
    ));

    // Worst case: both streams independently need their own shrink marker and their own
    // omission marker (up to 4 markers total), not just one of each combined.
    let marker_reserve =
        2 * (counter.count(P0_SHRINK_MARKER_TEXT) + counter.count(&omission_marker_text(999_999)));
    let min_p0_floor = counter.count(P0_SHRINK_MARKER_TEXT);

    if budget < marker_reserve + min_p0_floor {
        return Err(BudgetError::TooSmall {
            budget,
            minimum: marker_reserve + min_p0_floor,
        });
    }

    let available_for_mandatory = budget - marker_reserve;

    let (mandatory, optional): (Vec<Chunk>, Vec<Chunk>) =
        chunks.into_iter().partition(|c| c.priority == Priority::P0);

    let mandatory_tokens: usize = mandatory.iter().map(|c| c.token_count).sum();
    let (mandatory, shrunk_streams, dropped_mandatory_by_stream) =
        if mandatory_tokens > available_for_mandatory {
            shrink_mandatory(mandatory, available_for_mandatory, counter)
        } else {
            (mandatory, HashSet::new(), [0, 0])
        };
    let mandatory_ids: HashSet<ChunkId> = mandatory.iter().map(|c| c.id).collect();

    let included_mandatory_tokens: usize = mandatory.iter().map(|c| c.token_count).sum();
    let mut remaining = budget - included_mandatory_tokens - marker_reserve;

    let mut optional_sorted = optional;
    optional_sorted.sort_by(|a, b| b.score.total_cmp(&a.score));

    let mut selected: Vec<Chunk> = mandatory;
    let mut dropped_optional_by_stream = [0usize; 2];
    for chunk in optional_sorted {
        if chunk.token_count <= remaining {
            remaining -= chunk.token_count;
            selected.push(chunk);
        } else {
            dropped_optional_by_stream[stream_index(chunk.source.stream)] += 1;
        }
    }

    // Markers, per stream: a shrink marker if that stream had P0 content shrunk or
    // dropped, an omission marker if that stream had anything dropped at all.
    for stream in STREAMS {
        let idx = stream_index(stream);
        let stream_line_count = stream_lines(stream, stdout_lines, stderr_lines).len();
        if shrunk_streams.contains(&stream) {
            selected.push(marker_chunk(
                &mut next_id,
                stream,
                stream_line_count,
                P0_SHRINK_MARKER_TEXT.to_string(),
                counter,
            ));
        }
        let dropped_count = dropped_optional_by_stream[idx] + dropped_mandatory_by_stream[idx];
        if dropped_count > 0 {
            selected.push(marker_chunk(
                &mut next_id,
                stream,
                stream_line_count + 1,
                omission_marker_text(dropped_count),
                counter,
            ));
        }
    }

    let mut output = render(&selected);
    while count_output_tokens(&output, counter) > budget {
        match lowest_scoring_optional_index(&selected, &mandatory_ids) {
            Some(idx) => {
                selected.remove(idx);
                output = render(&selected);
            }
            None => return Err(BudgetError::Internal),
        }
    }

    Ok(output)
}

fn stream_index(stream: Stream) -> usize {
    match stream {
        Stream::Stdout => 0,
        Stream::Stderr => 1,
    }
}

fn stream_lines<'a>(stream: Stream, stdout: &'a [String], stderr: &'a [String]) -> &'a [String] {
    match stream {
        Stream::Stdout => stdout,
        Stream::Stderr => stderr,
    }
}

fn build_chunks(
    lines: &[String],
    stream: Stream,
    next_id: &mut usize,
    counter: &dyn TokenCounter,
) -> Vec<Chunk> {
    let total = lines.len();
    lines
        .iter()
        .enumerate()
        .map(|(line_number, text)| {
            let (kind, priority) = relevance::classify(text);
            let id = ChunkId(*next_id);
            *next_id += 1;
            Chunk {
                id,
                kind,
                token_count: counter.count(text),
                score: relevance::score(priority, line_number, total),
                priority,
                source: SourceMetadata {
                    stream,
                    line_number,
                },
                text: text.clone(),
                relationships: Vec::new(),
            }
        })
        .collect()
}

fn omission_marker_text(count: usize) -> String {
    if count == 1 {
        "[stk: omitted 1 line]".to_string()
    } else {
        format!("[stk: omitted {count} lines]")
    }
}

fn marker_chunk(
    next_id: &mut usize,
    stream: Stream,
    line_number: usize,
    text: String,
    counter: &dyn TokenCounter,
) -> Chunk {
    let id = ChunkId(*next_id);
    *next_id += 1;
    let token_count = counter.count(&text);
    Chunk {
        id,
        kind: ChunkKind::OmissionMarker,
        token_count,
        score: 0.0, // markers never enter the optional-selection sort; irrelevant here
        priority: Priority::P0,
        source: SourceMetadata {
            stream,
            line_number,
        },
        text,
        relationships: Vec::new(),
    }
}

/// Keeps the highest-scoring chunks first, truncating or dropping once budget runs out.
/// Returns the survivors, which streams had anything shrunk, and per-stream drop counts.
fn shrink_mandatory(
    mut mandatory: Vec<Chunk>,
    available: usize,
    counter: &dyn TokenCounter,
) -> (Vec<Chunk>, HashSet<Stream>, [usize; 2]) {
    mandatory.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut survivors = Vec::new();
    let mut shrunk_streams = HashSet::new();
    let mut dropped_by_stream = [0usize; 2];
    let mut remaining = available;
    for mut chunk in mandatory {
        if chunk.token_count <= remaining {
            remaining -= chunk.token_count;
            survivors.push(chunk);
            continue;
        }
        shrunk_streams.insert(chunk.source.stream);
        if remaining == 0 {
            dropped_by_stream[stream_index(chunk.source.stream)] += 1;
            continue; // no room left at all: drop entirely
        }
        chunk.text = truncate_to_fit(&chunk.text, remaining, counter);
        chunk.token_count = counter.count(&chunk.text);
        if chunk.token_count <= remaining {
            remaining -= chunk.token_count;
            survivors.push(chunk);
        } else {
            dropped_by_stream[stream_index(chunk.source.stream)] += 1;
        }
    }
    (survivors, shrunk_streams, dropped_by_stream)
}

/// Binary-searches for the longest head of `text` (plus a "...") that the *actual*
/// counter confirms fits within `remaining` -- a char-count inverse of the token formula
/// (e.g. `remaining*3`) can't be trusted directly, since `ceil(chars/3)` rounding means a
/// one-shot guess can come back costing one token more than `remaining`.
fn truncate_to_fit(text: &str, remaining: usize, counter: &dyn TokenCounter) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        let candidate: String = chars[..mid].iter().collect();
        if counter.count(&format!("{candidate}...")) <= remaining {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let kept: String = chars[..lo].iter().collect();
    format!("{kept}...")
}

fn lowest_scoring_optional_index(
    selected: &[Chunk],
    mandatory_ids: &HashSet<ChunkId>,
) -> Option<usize> {
    selected
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind != ChunkKind::OmissionMarker && !mandatory_ids.contains(&c.id))
        .min_by(|(_, a), (_, b)| a.score.total_cmp(&b.score))
        .map(|(idx, _)| idx)
}

fn render(selected: &[Chunk]) -> BudgetedOutput {
    let mut by_stream: [Vec<&Chunk>; 2] = [Vec::new(), Vec::new()];
    for chunk in selected {
        by_stream[stream_index(chunk.source.stream)].push(chunk);
    }
    for stream_chunks in &mut by_stream {
        stream_chunks.sort_by_key(|c| c.source.line_number);
    }
    let render_one = |chunks: &[&Chunk]| -> Vec<u8> {
        let mut out = String::new();
        for chunk in chunks {
            out.push_str(&chunk.text);
            out.push('\n');
        }
        out.into_bytes()
    };
    BudgetedOutput {
        stdout: render_one(&by_stream[0]),
        stderr: render_one(&by_stream[1]),
    }
}

fn count_output_tokens(output: &BudgetedOutput, counter: &dyn TokenCounter) -> usize {
    counter.count(&String::from_utf8_lossy(&output.stdout))
        + counter.count(&String::from_utf8_lossy(&output.stderr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::tokenizer::ApproximateCounter;

    fn lines(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn normal_budgeted_compression_keeps_the_error() {
        let stdout_lines = lines(&[
            "Compiling stk v0.1.0",
            "error[E0308]: mismatched types",
            "warning: unused variable",
            "note: some verbose detail nobody needs",
        ]);
        let out = select(&stdout_lines, &[], 100, &ApproximateCounter).unwrap();
        let rendered = String::from_utf8(out.stdout).unwrap();

        assert!(rendered.contains("error[E0308]"));
    }

    #[test]
    fn a_tiny_budget_forces_p0_to_shrink_and_carries_a_shrink_marker() {
        let long_error = format!("error: {}", "x".repeat(500));
        let stdout_lines = lines(&[&long_error]);

        // Big enough to pass the floor, far too small to hold the whole error.
        let out = select(&stdout_lines, &[], 65, &ApproximateCounter).unwrap();
        let rendered = String::from_utf8(out.stdout).unwrap();

        assert!(
            rendered.contains("[stk: P0 content truncated to fit budget]"),
            "expected a shrink marker, got: {rendered:?}"
        );
        assert!(
            rendered.len() < long_error.len(),
            "expected the error to actually be shortened"
        );
        let actual_tokens = ApproximateCounter.count(&rendered);
        assert!(
            actual_tokens <= 65,
            "rendered output must still honor the budget, got {actual_tokens} tokens"
        );
    }

    #[test]
    fn a_budget_below_the_floor_is_refused_not_rendered() {
        let result = select(&lines(&["error: anything"]), &[], 1, &ApproximateCounter);
        assert!(matches!(result, Err(BudgetError::TooSmall { .. })));
    }

    #[test]
    fn dropped_optional_content_leaves_a_visible_omission_marker() {
        let stdout_lines = lines(&[
            "error: the one thing that matters",
            "note: filler line one that is not important at all",
            "note: filler line two that is not important at all",
            "note: filler line three that is not important at all",
        ]);
        let out = select(&stdout_lines, &[], 65, &ApproximateCounter).unwrap();
        let rendered = String::from_utf8(out.stdout).unwrap();

        assert!(rendered.contains("error: the one thing that matters"));
        assert!(
            rendered.contains("[stk: omitted"),
            "expected an omission marker, got: {rendered:?}"
        );
    }

    #[test]
    fn nothing_dropped_means_no_omission_marker_at_all() {
        let stdout_lines = lines(&["error: small", "note: also small"]);
        let out = select(&stdout_lines, &[], 500, &ApproximateCounter).unwrap();
        let rendered = String::from_utf8(out.stdout).unwrap();

        assert!(!rendered.contains("[stk: omitted"));
        assert!(!rendered.contains("[stk: P0"));
    }

    #[test]
    fn both_streams_needing_their_own_shrink_marker_does_not_spuriously_error() {
        let long_error = format!("error: {}", "x".repeat(500));
        let stdout_lines = lines(&[&long_error]);
        let stderr_lines = lines(&[&long_error]);

        let result = select(&stdout_lines, &stderr_lines, 70, &ApproximateCounter);
        assert!(
            result.is_ok(),
            "two streams each independently needing a shrink marker must not exceed the \
             shared marker reserve and trip BudgetError::Internal, got: {result:?}"
        );
    }

    #[test]
    fn stdout_and_stderr_are_rendered_and_omitted_independently() {
        let stdout_lines = lines(&["error: stdout error"]);
        let stderr_lines = lines(&[
            "note: stderr filler one that nobody needs at all",
            "note: stderr filler two that nobody needs at all",
        ]);
        let out = select(&stdout_lines, &stderr_lines, 65, &ApproximateCounter).unwrap();
        let stdout_rendered = String::from_utf8(out.stdout).unwrap();
        let stderr_rendered = String::from_utf8(out.stderr).unwrap();

        assert!(stdout_rendered.contains("error: stdout error"));
        assert!(!stdout_rendered.contains("[stk: omitted"));
        assert!(stderr_rendered.contains("[stk: omitted"));
    }
}
