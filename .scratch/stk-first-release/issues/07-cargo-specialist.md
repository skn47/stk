# 07: Cargo `Specialist`

**What to build:** `stk cargo <subcommand> [args...]` dispatches to the Cargo `Specialist`, which parses Cargo's raw output into `Chunk`s recognizing compiler error codes, file/line/column references, warnings, build phases, test failures, and panic locations — then reuses ticket 06's `Budget`-bounded selection/rendering to produce a token-bounded, failure-preserving result.

**Blocked by:** 06

**Status:** done

- [x] `stk cargo check` / `stk cargo test` (and other Cargo subcommands) dispatch to the Cargo `Specialist` rather than the generic pipeline, via a new `specialists::lookup` registry checked in `cli.rs` after the built-in verbs.
- [x] Compiler error codes (e.g. `E0308`) and their file:line:column locations are extracted as high-priority (`P0`) `Chunk`s — verified against real `cargo check` output from a deliberately broken fixture project, not hand-imagined lines.
- [x] Warnings (`P2`), build-phase noise (`Compiling`/`Checking`/`Finished`/etc., `P4`), and successful test lines (`P4`) are classified at lower priority than errors/failures.
- [x] Panic locations, assertion detail lines, and failing-test lines are extracted as `P0` and preserved under `--budget`.
- [x] Fixture-driven tests cover a Rust type-error project (`tests/fixtures/cargo_projects/type_error/`, preserves the error code + location, unbounded and under a tight budget) and a passing-build-with-warning project (`.../passing_with_warning/`, compresses away routine `Compiling`/`Finished` noise).
- [x] A golden-comparison test confirms `stk cargo check` retains the mandatory evidence (error code + location) real `rtk cargo check` does, on the same fixture project (skipped gracefully without `rtk`).

**Implementation notes:**
- The Budget Selection Algorithm (`budget::selector::select`) now takes a `Classifier` (`fn(&str) -> (ChunkKind, Priority)`) parameter instead of hardcoding the generic one, so `compile` and specialist dispatch share the exact same selection/shrink/render machinery with their own classification rules. Shared "execute a command, apply fast-path-or-budget, write output" logic moved from `compile.rs` into a new top-level `compression` module (not under `verbs/`, since it isn't itself a verb).
- The Cargo classifier's rules were derived from real `cargo check`/`build`/`test` output captured against deliberately broken/warning/failing throwaway projects, not assumption — matching this session's established pattern of verifying claims about real tools empirically.
- Code review found no correctness bugs (two independent passes, including a forked re-check of the classify-heuristic ordering, priority-tier scoring interaction, and CLI dispatch edge cases). Two conventions findings were fixed: the Cargo classifier's doc comment exceeded the repo's 3-line convention (trimmed), and the new shared `compression` module lived under `src/verbs/` despite not being a verb itself (moved to a top-level `src/compression.rs`).
