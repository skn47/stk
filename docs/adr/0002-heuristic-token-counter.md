# First-release token counter is a dependency-free heuristic, not a real tokenizer

STK's token budget is explicitly a guarantee against STK's own counter, not against the true tokenizer of whatever model consumes the output. We considered using a real BPE tokenizer (`tiktoken-rs`, HuggingFace `tokenizers`) for accuracy, but that would implicitly anchor STK's "budget" to one vendor's specific encoding — undermining the model-agnostic framing — and adds a dependency/latency cost for a guarantee that doesn't need model-exact counts, only a conservative over-estimate.

First release: `count(text) = ceil(max(chars(text)/3, words(text)*1.4))`.

## Consequences

The `TokenCounter` trait keeps this swappable; a `ModelTokenizer` implementation can be added later for users who want tighter alignment to a specific downstream model, without changing the budget contract itself.
