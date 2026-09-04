# 07: Cargo `Specialist`

**What to build:** `stk cargo <subcommand> [args...]` dispatches to the Cargo `Specialist`, which parses Cargo's raw output into `Chunk`s recognizing compiler error codes, file/line/column references, warnings, build phases, test failures, and panic locations — then reuses ticket 06's `Budget`-bounded selection/rendering to produce a token-bounded, failure-preserving result.

**Blocked by:** 06

**Status:** ready-for-agent

- [ ] `stk cargo check` / `stk cargo test` (and other Cargo subcommands) dispatch to the Cargo `Specialist` rather than the generic pipeline.
- [ ] Compiler error codes (e.g. `E0308`) and their file:line:column locations are extracted as high-priority `Chunk`s.
- [ ] Warnings, build-phase noise, and successful test/build output are classified at lower priority than errors/failures.
- [ ] Panic locations and test failures are extracted and preserved under `--budget`.
- [ ] Fixture-driven tests cover at least: a Rust type-error fixture (must preserve the error code + location) and a passing-build-with-noise fixture (must compress away routine success output).
- [ ] A golden-comparison test confirms `stk cargo check` retains at least the mandatory evidence `rtk cargo check` does, on the same fixture (skipped gracefully without `rtk`).
