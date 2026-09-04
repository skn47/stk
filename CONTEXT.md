# stk

`stk` is a stateful context compiler for coding agents: it turns noisy shell/tool output into the smallest high-value context an agent needs, within an explicit token budget. This glossary covers terms specific to STK's own domain — see `../stk_implementation_overview.md` for the full design.

## Language

**Chunk**:
A scored, budgetable unit of output — a single error, warning, source range, stack frame, or omission marker. Everything STK selects into or excludes from the final rendered output is a Chunk, including omission markers themselves (`ChunkKind::OmissionMarker` — see ADR-0001).
_Avoid_: fragment, block, segment

**Specialist**:
Command-aware logic that parses one tool's raw output (Cargo, Git, ...) into Chunks using that tool's own semantics. Distinct from the generic pipeline (`stk compile`), which has no specialist.
_Avoid_: parser, plugin (plugin implies runtime-loaded; specialists are compiled in)

**Budget**:
The absolute token ceiling STK renders output within, guaranteed against STK's own token counter (see `TokenCounter`) — not a guarantee against any specific downstream model's real tokenizer.
_Avoid_: limit, quota

**Session** / **Session Store**:
The temporal-memory mechanism (Phase 4) that remembers command history, error locations, and touched files across invocations sharing a repo root + cwd. Does not exist in the first release — no session-store code runs, not even a stub, until Phase 4 lands.
_Avoid_: cache, state (too generic — reserve "Session" for this specific temporal-memory concept)

**Intent**:
The task-purpose hint (`debug`, `docs`, `test`, `review`, `security`) that steers relevance scoring, shipping in Phase 6. Until then, `--intent` is a rejected/unrecognized CLI flag — never silently accepted or ignored.
_Avoid_: mode, profile

**TokenCounter**:
STK's pluggable token-counting abstraction. The first-release implementation is a conservative, dependency-free heuristic (over-estimates, never under-estimates) — not a real model tokenizer (see ADR-0002).
_Avoid_: tokenizer (reserve for a future exact/model-specific `TokenCounter` implementation, to keep it distinct from the abstraction itself)
