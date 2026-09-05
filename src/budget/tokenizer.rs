/// STK's token-counting abstraction. The `Budget` guarantee is against this counter's
/// own count, not any specific downstream model's real tokenizer (see ADR-0002).
pub trait TokenCounter {
    fn count(&self, text: &str) -> usize;
}

/// First-release `TokenCounter`: `ceil(max(chars/3, words*1.4))`, a conservative,
/// dependency-free heuristic that over-counts rather than under-counts (ADR-0002).
pub struct ApproximateCounter;

impl TokenCounter for ApproximateCounter {
    fn count(&self, text: &str) -> usize {
        let chars = text.chars().count() as f64;
        let words = text.split_whitespace().count() as f64;
        (chars / 3.0).max(words * 1.4).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_counts_as_zero() {
        assert_eq!(ApproximateCounter.count(""), 0);
    }

    #[test]
    fn short_word_heavy_text_uses_the_word_based_estimate() {
        // "a a a a a a a a a a" -- 10 tiny words: chars/3 underestimates, words*1.4 doesn't.
        let text = "a ".repeat(10);
        let by_chars = (text.chars().count() as f64 / 3.0).ceil() as usize;
        let count = ApproximateCounter.count(&text);
        assert!(
            count > by_chars,
            "word-based estimate should dominate for many tiny words"
        );
    }

    #[test]
    fn long_unbroken_text_uses_the_char_based_estimate() {
        // One giant "word": words*1.4 rounds to ~1, chars/3 dominates.
        let text = "x".repeat(3000);
        assert_eq!(ApproximateCounter.count(&text), 1000);
    }

    #[test]
    fn never_undercounts_a_simple_known_case() {
        // "hello world" is 2 tokens by any real tokenizer's count; our estimate must be >= that.
        assert!(ApproximateCounter.count("hello world") >= 2);
    }
}
