# 02: `stk run` — raw passthrough + process-wrapper contract

**What to build:** `stk run [ARGS...]` executes the given command as a raw `sh -c`-style passthrough: no filtering, no tracking. It must be indistinguishable from running the command directly, including under Ctrl-C — this is the escape hatch, not STK's own pipeline. This ticket also stands up the fixture corpus directory and a golden-comparison harness (run each fixture through both the real `rtk` and `stk`, skipped gracefully when `rtk` isn't installed) — kept minimal here since `run` is byte-identical passthrough, the easiest verb to prove equivalence for.

**Blocked by:** 01

**Status:** done

- [x] `stk run <cmd> [args...]` produces byte-identical stdout, stderr, and exit code to running `<cmd> [args...]` directly. Positional args are exec'd directly (no shell re-parsing, avoiding quoting loss); `stk run -c "shell string"` is the explicit `sh -c` form, matching the real `rtk run`'s own `-c`/positional split.
- [x] stdout/stderr are streamed through live and unbuffered: `execute_inherited` uses `Stdio::inherit()` so the child's fds are the real terminal's fds directly, with no buffering copy loop in our own code.
- [x] stdin is passed through live to the child (also via `Stdio::inherit()`).
- [x] SIGINT/SIGTERM sent to `stk run` are forwarded to the child via a background signal-handling thread (`SignalForwarder`); `stk run` waits for the child to exit and propagates its exit code, or exits `128+signal` if the child was signal-terminated.
- [x] A fixture-corpus directory (`tests/fixtures/`) and test-support helpers (`tests/support/fixture.rs`, `tests/support/rtk.rs`) exist; fixtures are TOML (`command`, `must_preserve`, `may_remove`).
- [x] A golden-comparison test runs the `echo_hello` fixture through both `stk run` and the real installed `rtk run`, asserting identical output; skipped (not failed) when `rtk` isn't present.

**Implementation notes:**
- The golden-comparison and signal-forwarding tests spawn the compiled `stk` binary as a real subprocess (not the in-process `cli::run` seam) — genuine OS-level passthrough fidelity can only be verified that way, since `execute_inherited` deliberately bypasses the seam's stdout/stderr sinks in favor of real fd inheritance. Dispatch/arg-parsing logic (which command/args reach the executor, `-c` vs positional, exit-code-from-signal mapping) is still tested in-process via `FakeExecutor` in `src/verbs/run.rs`.
- Code review caught and fixed: a child-process leak if `SignalForwarder::spawn` failed after the child was already spawned (now degrades to no forwarding instead of erroring out before `wait()`); a PID-reuse race between the forwarder thread and the reaped child (narrowed via an `AtomicBool` the executor sets right after `wait()`, though not fully eliminated — POSIX PID-based signaling has no complete fix short of `pidfd`, judged disproportionate for this ticket); duplicated signal-extraction code between `execute`/`execute_inherited` (extracted to a helper); a racy fixed-sleep in the SIGTERM test (widened, documented as a judged trade-off rather than fully deterministic); an over-long doc comment; and a comment referencing "future tickets" instead of the concept itself.
