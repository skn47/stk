# 01: Foundation — CLI seam + unsupported-command dispatch

**What to build:** A working `stk` binary whose CLI entry point already dispatches through the confirmed test seam, so every later verb is added by extending one real code path instead of stubbing around it. The only real behavior at this point: any command that doesn't match a known verb or registered `Specialist` fails with a clear "unsupported command" error and a nonzero exit code — matching the "no silent fallback to `compile`" rule.

**Blocked by:** None (can start immediately)

**Status:** done

- [x] `cli::run(args, stdin, stdout: &mut dyn Write, stderr: &mut dyn Write, executor) -> i32` (the exit code) exists as a library-level function callable in-process from tests, independent of `main.rs`. Output is written to the `stdout`/`stderr` sinks as it becomes available, not returned as a fully-buffered value — this is what lets `run`/`proxy` (ticket 02) stream live without a signature change later.
- [x] A `CommandExecutor` trait abstracts child-process spawning, with a `RealExecutor` implementation used by `main.rs` and a fake/test implementation (`FakeExecutor`) available to tests that returns a canned exit code/stdout/stderr for a given invocation and records every invocation it received.
- [x] `main.rs` is a thin wrapper: real `argv`, real stdin/stdout/stderr, `RealExecutor`, exits with the exit code `cli::run` returns.
- [x] Invoking `stk` with an unrecognized top-level command (no matching verb, no registered `Specialist`) returns a clear "unsupported command" error on stderr and a nonzero exit code — no silent fallback to any other pipeline.
- [x] At least one test calls `cli::run` directly with a fake executor and asserts the unsupported-command behavior, proving the seam works end-to-end without spawning a real process.

**Implementation notes:**
- `ExecutionResult` also carries `terminating_signal: Option<i32>` (populated via `ExitStatusExt::signal()` on Unix) so `run`/`proxy` (tickets 02/03) can compute the spec's `128+signal` exit code later without a breaking struct change.
- A bare `stk` with no command gets a distinct "no command given" message rather than being folded into the "unsupported command" error.
- Code review (background `/code-review` pass) caught and fixed: signal loss in `RealExecutor`, a lossy `as u8` exit-code cast in `main.rs` (now clamped), the no-command message issue above, an over-long doc comment, and a duplicate test.
