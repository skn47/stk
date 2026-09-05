# 05: Fast path — `stk compile` unbounded (Janitor + Aggregator)

**What to build:** `stk compile` (given a command to wrap, or reading stdin when none is given) runs the Janitor + Aggregator fast path: ANSI escape codes stripped, redundant whitespace/excess blank lines normalized, known boilerplate filtered, and exactly-repeated lines collapsed into `[repeated Nx]`. No `Budget`/priority/omission-marker logic yet — this ticket proves the deterministic cleanup pipeline alone, before selection logic is layered on top.

**Blocked by:** 04

**Status:** done

- [x] `stk compile <cmd>` and `cat file | stk compile` both work (executing a command vs. reading stdin when none is given). Deliberately not combinable: a command's own stdin isn't forwarded (matches the design doc's "reads stdin *instead of* executing a command" framing) — `stk run`/`stk proxy` are for a command that itself needs piped stdin.
- [x] ANSI escape codes are stripped from the output (`normalize::ansi`, reused from ticket 04).
- [x] Redundant whitespace and excessive empty lines are normalized (`normalize::whitespace`: trailing-whitespace trim + blank-line-run collapse).
- [x] A configurable set of known-boilerplate patterns is filtered out (`normalize::boilerplate`: pure visual-separator lines for now — generic-pipeline scope only, not command-specific).
- [x] Exactly-repeated consecutive lines are collapsed to a `<line> [repeated Nx]` form (`aggregation::repetition`).
- [x] Unreasonably long lines are safeguarded against (`normalize::long_lines`: head/tail-preserving truncation past 2000 chars).
- [x] Fixture-driven tests cover an ANSI-heavy log, a repeated-retry log, and a log with long lines (`tests/fixtures/{ansi_heavy,repeated_retries,long_lines}.toml`) — asserting the cleaned-up shape via `tests/compile_verb.rs`, spawning the real binary per the established convention.

**Implementation notes:**
- Pipeline order matters: `repetition::collapse` must run on the true, unmodified line sequence *before* `boilerplate::filter` or `long_lines::truncate`, or those lossy steps can fabricate false adjacency/equality that never existed in the real output (see code review below). `aggregation::repetition::strip_marker` lets boilerplate filtering still recognize a collapsed separator run (`---------- [repeated 2x]`) as boilerplate despite running after collapsing.
- New `ExecutionResult::exit_info()` projects the buffered-executor result down to an `ExitInfo` so `compile` reuses `to_process_exit_code()` rather than reimplementing the 128+signal mapping.
- Code review caught and fixed 6 issues: two related false-positive bugs where removing/truncating content *before* checking for repeats fabricated repeat counts that never existed (reordered the pipeline, regression-tested, verified against the built binary); a broken pipe on stdout was swallowing the real child exit code entirely (a CI script checking `$?`/`PIPESTATUS` would see false success for a failed build — fixed to still return the child's real exit code); an unnecessary `Vec<char>` allocation on every short line in `long_lines::truncate` (added a cheap byte-length pre-check); the stdin-not-forwarded-to-a-wrapped-command behavior lacked a documenting comment explaining it's deliberate, not an oversight; and `tests/compile_verb.rs` mixed in-process `FakeExecutor` tests with fixture-driven ones, breaking the established convention (every other verb's `tests/*_verb.rs` spawns the real binary) — fixed by rewriting the fixture tests to spawn the real binary and dropping the two tests that duplicated inline unit-test coverage already in `src/verbs/compile.rs`.
