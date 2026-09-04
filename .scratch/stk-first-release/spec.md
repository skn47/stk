# stk: First Release

Status: ready-for-agent

Source design doc: `/home/h/stk_implementation_overview.md` (three-entry §1.1 Review Log; scope defined in §26). Domain vocabulary: `/home/h/stk/CONTEXT.md`. Related decisions: `/home/h/stk/docs/adr/0001-omission-markers-are-chunks.md`, `/home/h/stk/docs/adr/0002-heuristic-token-counter.md`.

## Problem Statement

A developer driving a coding agent burns agent context-window tokens on raw, noisy tool output — compiler errors buried in build noise, Git status wrapped in decoration, test failures mixed into thousands of lines of passing-test ceremony. Existing stateless compressors (`rtk`) treat every invocation of a command identically regardless of what actually matters right now, and give no hard guarantee about how large their output can get: a percentage reduction on a huge input can still leave a huge output.

## Solution

`stk` is a CLI a developer runs instead of the raw command (or pipes raw output into) that renders that command's output as a token-**Budget**-bounded, relevance-ranked representation: it preserves mandatory failure evidence, aggressively compresses everything else, and always marks what it removed rather than pretending the output was complete. It ships as a close alias of `rtk`'s CLI surface for the verbs that carry over directly (so switching is low-friction), while introducing `stk compile` as its own generic, scored, budgeted pipeline, and `Specialist`-aware handling for Cargo and Git specifically.

## User Stories

### CLI dispatch & migration from `rtk`

1. As a developer migrating from `rtk`, I want `stk init [--global|-g] [--codex]` to install shell/agent integration idempotently and report exactly which files it changed, so that adopting `stk` feels familiar and safe to re-run.
2. As a developer migrating from `rtk`, I want `stk proxy [ARGS...]` to execute the child command transparently (byte-identical stdout/stderr/exit code) while tracking usage silently, so that I get adoption data with zero behavior change.
3. As a developer, I want `stk run [ARGS...]` to be a raw, unfiltered `sh -c`-style passthrough with no tracking, so that I have an escape hatch identical to `rtk run` when I explicitly don't want any compression.
4. As a developer, I want `stk pipe [-f FILTER] [--passthrough]` to read stdin, apply one named filter, and print the result, so that I can drop `stk` into existing Unix pipelines the same way I use `rtk pipe`.
5. As a developer, I want `stk COMMAND [ARGS...]` (e.g. `stk cargo check`) to dispatch to a registered `Specialist`, and to fail loudly with an "unsupported command" error when no `Specialist` is registered for `COMMAND`, so that I never get a silently-different (and possibly worse) result from an implicit fallback.
6. As a developer, I want `stk gain [-p] [-H] [-g] [-q] [-t TIER] [-d|-w|-m|-a] [-f FORMAT] [-F] [--reset]` to show a token-savings summary/history compatible with the documented subset of `rtk gain`'s flags, so that I can see whether adopting `stk` is paying off.
7. As a developer, I want `stk config [--create]` to show or create the STK config file, so that I can inspect and start customizing my setup the same way I would with `rtk config`.
8. As a developer, I want `stk bench --against rtk` to compare `stk` and the real locally-installed `rtk` binary on identical fixtures/budgets, so that I have concrete, reproducible evidence of whether switching is worthwhile.
9. As a developer, I want global STK options (`--budget`, `--intent`, `--explain`, etc.) recognized only before the subcommand — matching `rtk`'s own convention — so that my existing muscle memory for flag placement transfers directly.
10. As a developer, I want `stk --intent ...` and `stk --explain ...` to be rejected as unrecognized flags in this release (not silently accepted or ignored), so that I am never misled into thinking intent-aware scoring or explainability is active when it isn't.
11. As a developer, I want config precedence to be `CLI flags > environment variables > repository/project config file > global config file`, with `stk init --codex` writing only to ordinary project config (no separate runtime "Codex-detection" layer), so that config resolution is simple and matches what `rtk` actually does today.

### Process-wrapper contract

12. As a developer, I want `run`, `proxy`, `compile`, and specialist dispatch to all forward SIGINT/SIGTERM to the child process, wait for it to exit, and propagate its exit code (or `128+signal` if it was signal-terminated), so that `stk` behaves like a transparent wrapper under Ctrl-C and process managers.
13. As a developer, I want `run` and `proxy` to stream stdout/stderr through live and unbuffered, so that long-running or interactive-ish (non-TTY) commands still show progress in real time.
14. As a developer, I want `compile` and specialist dispatch to buffer the child's output so it can be compressed, and to handle a broken pipe on STK's own stdout gracefully (without corrupting or hanging), so that piping compressed output into something like `head` doesn't crash `stk`.
15. As a developer, I want stdin passed through live for `run`/`proxy`, and I want `stk pipe` (or `stk compile` invoked with no command) to be the way I feed data on stdin into the compression pipeline, so that stdin's role never depends on which verb I happen to be using.
16. As a developer, I acknowledge that PTY allocation and interactive child prompts are out of scope for this release, so that I don't expect `stk` to wrap a genuinely interactive program correctly yet.

### Fast path (Janitor + Aggregator)

17. As a coding agent, I want ANSI escape codes stripped from captured output, so that color codes and cursor-movement sequences never pollute what I read.
18. As a coding agent, I want redundant whitespace and excessive empty lines normalized away, so that vertical noise doesn't cost me tokens.
19. As a coding agent, I want known command-boilerplate (routine success chatter) filtered out before scoring even runs, so that the cheap wins happen first and cheaply.
20. As a coding agent, I want exactly-repeated lines collapsed into a `[repeated Nx]` summary, so that a retry storm doesn't drown out the one message that actually explains the failure.
21. As a coding agent, I want unreasonably long lines safeguarded (not left to blow the budget on their own), so that one pathological line can't defeat compression entirely.

### `Specialist`s: Cargo and Git

22. As a coding agent, I want the Cargo `Specialist` to recognize compiler error codes, file/line/column references, warnings, build phases, test failures, and panic locations, so that a `cargo check`/`cargo test` failure is preserved precisely, not approximately.
23. As a coding agent, I want the Git `Specialist` to recognize modified/staged files, conflict markers, diff hunks, rename information, and branch divergence, so that `git status`/`git diff` output stays structurally meaningful after compression.
24. As a developer, I want commands with no registered `Specialist` and no explicit `compile`/`run`/`proxy` verb to fail with a clear "unsupported command" error rather than guessing, so that I always know whether I'm getting specialist-aware handling or not.

### `stk compile`: the generic pipeline

25. As a coding agent, I want `stk compile` to normalize, aggregate, score, and budget arbitrary command output (or piped stdin) with no command-specific `Specialist`, so that I have one general-purpose compression entry point for anything `stk` doesn't have bespoke support for.
26. As a coding agent, I want `cat huge.log | stk compile` (no command given) to read from stdin, so that I can compress arbitrary captured logs, not just live command output.

### First-class filters

27. As a coding agent, I want `stk read`, `grep`, `find`, `log`, `err`, `summary`, `diff`, and `json` to be native STK implementations (not thin wrappers proxying to `ls`/`rg`/`find`/etc. the way `rtk`'s versions do), so that their output goes through the same `Budget`/`Chunk` model as everything else.
28. As a coding agent, I want these filters to be explicitly stateless in this release (no `Session`-aware behavior), so that their behavior is predictable and doesn't silently change based on command history that doesn't exist yet.
29. As a coding agent, I want `stk read src/main.rs --budget N` to render a token-bounded view of a source file using conservative text-based slicing (no Tree-sitter yet), so that I can read large files without blowing my context budget, understanding that function-boundary precision arrives in a later milestone.

### `Budget` and the Hard `Budget` Invariant

30. As a coding agent, I want `stk --budget N <command>` to produce the highest-value representation of that command's output that fits within approximately N tokens, so that I have a predictable, absolute ceiling on tool-result size regardless of the raw output's actual length.
31. As a coding agent, I want `Chunk`s classified by priority (P0 fatal errors/exact failure location, down through P5 repeated/noisy output), with P0 content included first and optional content packed by score into whatever budget remains, so that the most important information always survives compression first.
32. As a coding agent, I want a final render-and-recount pass after chunk selection (never trusting chunk-level token estimates alone), so that the promise "fits in budget" is a promise about what I actually receive, not an approximation.
33. As a coding agent, I want mandatory (P0) content structurally shrunk — never silently dropped, and never allowed to silently blow the budget — when it alone exceeds the budget, with a visible `[stk: P0 content truncated to fit budget]`-style marker whenever that happens, so that I always know when even the "must keep" content had to be cut down.
34. As a developer, I want `stk` to refuse a `--budget` value below the minimum viable floor (marker reserve + smallest shrunk-P0 representation) with a clear error, rather than attempt to render degenerate output, so that I never get a technically-fits-the-number response that's actually useless or broken.
35. As a coding agent, I want STK's token counter to be conservative — biased toward over-counting, never under-counting — using a fast, dependency-free heuristic (`ceil(max(chars/3, words*1.4))`), so that "fits within budget by STK's count" carries a real safety margin against whatever real tokenizer the downstream model actually uses.

### Omission markers

36. As a coding agent, I want compressed output to explicitly say when content was removed (e.g. `[stk: omitted 1,482 repeated/success lines]`) rather than silently look complete, so that I know more context exists and can go get it if I need to.
37. As a coding agent, I want an omission marker to be accounted for using the same selection/scoring machinery as every other `Chunk` (not a separate bookkeeping path bolted on at render time), so that the "markers count against the budget too" guarantee is structurally true, not just documented.

### Testing & compatibility

38. As a maintainer, I want a fixture corpus (Cargo failures, Rust borrow-checker errors, pytest/Jest failures, Git diffs/conflicts, large logs, repeated retries, malformed source, etc.) with `must_preserve`/`may_remove` expectations per fixture, so that heuristic changes are regression-tested against real-world shapes of output.
39. As a maintainer, I want a golden-output comparison suite that runs identical fixtures through both the real local `rtk` and `stk` for every aliased verb, asserting `stk`'s output is equivalent-or-better and that mandatory evidence always survives, so that I have concrete proof `stk` is a safe replacement, not just a plausible one.

## Implementation Decisions

**Test seam** (confirmed with the user; signature refined during ticket 01's implementation — see note below): all first-release behavior is exercised through exactly one seam — a library-level entry point, `cli::run(args: &[String], stdin: impl Read, stdout: &mut dyn Write, stderr: &mut dyn Write, executor: &dyn CommandExecutor) -> i32` (the exit code) — called by a thin `main.rs` that supplies real `argv`, real stdin/stdout/stderr, and a `RealExecutor`. Tests call `cli::run` in-process with a fake `CommandExecutor` that returns canned stdout/stderr/exit-code for a given child command (no real `cargo`/`git` subprocess spawned in tests), or feed literal stdin for `compile`/`pipe`, and pass `Vec<u8>` buffers as the `stdout`/`stderr` sinks to inspect afterward. No lower seams (scorer, budgeter, specialists) are exposed directly to tests; every behavior, including Hard `Budget` Invariant edge cases, is reached through crafted fixtures at this one boundary. Filesystem-based filters (`read`/`grep`/`find`) are the one exception: they don't spawn a child process, so they're tested against real temp-directory fixture files rather than through the executor.

*Note on the signature*: the originally-confirmed shape returned a fully-buffered `Output { exit_code, stdout, stderr }`. Implementing ticket 01 surfaced a real conflict with ticket 02's "stream live and unbuffered" requirement for `run`/`proxy` — a function that only returns output once finished can't stream it live to the real terminal in the meantime, no matter what happens internally. The fix keeps the one-seam, fake-executor testing philosophy intact and only changes *how* output reaches the caller: writer parameters instead of a buffered return value. Production passes real stdio (so streaming stays live); tests pass in-memory buffers and inspect them after the call returns.

**Modules built this release** (repo-relative, under `src/`, per the design doc's §16 layout, restricted to first-release scope):
- `main.rs`, `cli.rs` — argument parsing, subcommand dispatch, and the `cli::run` seam itself
- `capture/` — process wrapping (`process.rs`), the `CommandExecutor` trait plus its `RealExecutor` implementation (new relative to §16 — this is the seam introduced by this spec), and stdin handling (`stream.rs`)
- `normalize/` — ANSI stripping (`ansi.rs`), whitespace cleanup (`whitespace.rs`)
- `specialists/` — `mod.rs` (registry, "unsupported command" error), `generic.rs`, `cargo.rs`, `git.rs` only (pytest/Node specialists are out of scope this release, despite appearing in the design doc's Phase 2 list — see Out of Scope)
- `structural/` — `stack_trace.rs`, `log_parser.rs` (regex/line-based chunking for Cargo/Git output; `tree_sitter.rs` is out of scope this release)
- `aggregation/` — `repetition.rs` (exact-match and streaming-window repeated-line detection)
- `scoring/` — `relevance.rs` only. The formula's `temporal_relevance` and `intent_relevance` terms are absent this release (no `Session`, no `Intent` — see below), and `recency` is reinterpreted as within-single-invocation positional recency (e.g. tail of a build log), not cross-command recency, since that requires `Session` state that doesn't exist yet. No `intent.rs` module.
- `budget/` — `tokenizer.rs` (`ApproximateCounter` per ADR-0002), `selector.rs` (the §20 selection algorithm, including the minimum-viable-budget floor check and the render-and-recount trim loop)
- `render/` — `compact.rs` only (no `explain.rs` this release)
- `config/` — `settings.rs` (TOML config, precedence chain per the design doc's patched §17.3, no runtime Codex-detection layer)

**Explicitly not built this release**: `context/` (no `session.rs`, `events.rs`, or `decay.rs` — not even inert stubs; see the design doc's §15 note and ADR context), `structural/tree_sitter.rs`, `render/explain.rs`, `scoring/intent.rs`.

**`Chunk` model**: `Chunk { id, kind, text, source, token_count, score, priority, relationships }`, with `ChunkKind` including `OmissionMarker` as a first-class variant (ADR-0001) alongside `Error`, `Warning`, `StackFrame`, `SourceFunction`, `SourceClass`, `SourceWindow`, `TestFailure`, `TestSuccess`, `DiffHunk`, `LogGroup`, `CommandSummary`, `Documentation`, `Boilerplate`. An omission marker's rendered text is its `Chunk` payload; it is selected, scored, and counted through the exact same path as every other chunk.

**`Budget` selection**: implements the design doc's §20 algorithm exactly — reserve `marker_reserve`, error below `marker_reserve + min_p0_floor`, shrink P0 structurally (with a truncation marker) if it alone exceeds budget, pack optional chunks by score into what remains, then render-and-recount and drop lowest-scoring optional chunks first if the render pass pushed over budget. An unrecoverable overflow after all optional chunks are dropped is an internal error, not a silent truncation.

**CLI flag vs. config-key handling for deferred features**: `--intent`/`--explain` are hard parse errors (unrecognized flag) at the CLI layer. By contrast, the `intent`, `session_memory`, and `session_ttl_minutes` *config file* keys are accepted and parsed but inert (forward-compatible schema, no behavior yet) — config files are not re-invoked per command the way flags are, so silently ignoring a forward-declared key doesn't create the same false-signal risk that accepting a flag would.

## Testing Decisions

- Tests assert only on external behavior observable through the `cli::run` seam: exit code, rendered stdout, rendered stderr (including where the `--explain`-style diagnostic channel would go is irrelevant this release, since `--explain` doesn't exist yet). No test reaches into `Chunk` construction, scoring internals, or the budget selector directly.
- The fixture corpus (design doc §24) is the primary testing asset: each fixture pairs a canned `CommandExecutor` response (or literal stdin) with `must_preserve` / `may_remove` expectations, run through `cli::run` at a given `--budget`, and asserted against the rendered output.
- A separate golden-output comparison suite (design doc §17.4) runs the same fixtures through both `stk` and the real installed `rtk` binary (skipped gracefully, not failed, when `rtk` isn't present in the test environment — e.g. CI without it installed) to assert equivalence-or-better and full mandatory-evidence retention for every aliased verb.
- Hard `Budget` Invariant edge cases (budget below the floor, P0 forced to shrink, full-optional-trim-and-still-over-budget) are tested by crafting fixtures whose raw size and `--budget` value deliberately trigger each branch of the §20 algorithm through `cli::run` — not via direct unit tests of the selector.
- Prior art: none in this repo yet (pre-implementation); the fixture/golden-comparison approach mirrors what the design doc already specifies normatively in §17.4 and §24, so it isn't a new invention, just the first implementation of an already-agreed pattern.

## Out of Scope

- Temporal memory (`stk memory show`/`stk memory reset`, `Session`/`Session Store`, session identity/decay/TTL) — Phase 4. No session-store code exists this release, not even an inert stub.
- Tree-sitter structural slicing (`structural/tree_sitter.rs`, grammar-aware function/class abstraction) — Phase 5. Source-file compression (`stk read`) uses conservative text-based slicing only.
- The Intent Engine (`--intent`, `scoring/intent.rs`, intent-specific scoring weights, config presets) and intent auto-detection — Phase 6. `--intent` is a hard parse error this release.
- Explainability (`--explain`, `stk inspect`, per-chunk score reporting) — Phase 7.
- Cross-file/cross-symbol context graph — Phase 8.
- `stk discover` and `stk session` (`rtk`'s Claude-Code adoption-mining commands) — aliased in name only, not implemented this release.
- pytest and Node test-runner `Specialist`s — present in the design doc's original Phase 2 list, but excluded from the first-release scope defined in §26; only Cargo and Git ship this release.
- PTY allocation and interactive (TTY-requiring) child processes.
- Any config-file schema beyond what `settings.rs` needs for this release's precedence chain (the `intent`/`session_memory`/`session_ttl_minutes` keys are parsed-and-ignored placeholders, not implemented behavior).

## Further Notes

- The design doc's Recommended Implementation Order (§32) — measurement harness → Janitor/Aggregator → `Specialist`s → chunk representation → absolute token budgeting — is the suggested build order once this spec is split into tickets.
- `rtk` compatibility claims throughout this spec were checked against the real, locally installed `~/.cargo/bin/rtk`, v0.42.4. If `rtk` is upgraded before or during implementation, re-verify flag-for-flag claims (`init`, `gain`) against its current `--help` output rather than trusting this spec's snapshot.
- Three §1.1 Review Log entries in the design doc (2026-09-03 ×2, 2026-09-04) record every design decision this spec depends on; when in doubt about *why* a decision was made, that log — not this spec — has the reasoning.
