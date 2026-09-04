# 02: `stk run` — raw passthrough + process-wrapper contract

**What to build:** `stk run [ARGS...]` executes the given command as a raw `sh -c`-style passthrough: no filtering, no tracking. It must be indistinguishable from running the command directly, including under Ctrl-C — this is the escape hatch, not STK's own pipeline. This ticket also stands up the fixture corpus directory and a golden-comparison harness (run each fixture through both the real `rtk` and `stk`, skipped gracefully when `rtk` isn't installed) — kept minimal here since `run` is byte-identical passthrough, the easiest verb to prove equivalence for.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] `stk run <cmd> [args...]` produces byte-identical stdout, stderr, and exit code to running `<cmd> [args...]` directly via `sh -c`.
- [ ] stdout/stderr are streamed through live and unbuffered as the child produces them (not buffered until exit).
- [ ] stdin is passed through live to the child.
- [ ] SIGINT/SIGTERM sent to `stk run` are forwarded to the child; `stk run` waits for the child to exit and propagates its exit code, or exits `128+signal` if the child was signal-terminated.
- [ ] A fixture-corpus directory and test runner exist (raw input/command, `must_preserve`/`may_remove` expectations) and can run a fixture through `cli::run`.
- [ ] A golden-comparison test runs at least one fixture through `stk run` and the real installed `rtk run`, asserting identical output; the test is skipped (not failed) when `rtk` isn't present in the environment.
