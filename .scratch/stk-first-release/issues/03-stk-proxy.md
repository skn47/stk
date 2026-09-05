# 03: `stk proxy` — tracked passthrough

**What to build:** `stk proxy [ARGS...]` behaves exactly like `stk run` — byte-identical stdout/stderr/exit code — but additionally records usage silently (no stderr summary printed), so developers who want `stk` in front of every command without any behavior change get adoption data for free.

**Blocked by:** 02

**Status:** done

- [x] `stk proxy <cmd> [args...]` produces byte-identical output/exit code to running the command directly, reusing `execute_inherited` (passthrough/signal-forwarding/streaming) from `stk run`. Unlike `run`, `proxy` has no `-c` form — verified against the real `rtk proxy --help` (positional args only) and confirmed empirically: `rtk proxy -c "echo hi"` tries to exec a literal file named `-c` and fails, which `stk proxy` now matches exactly.
- [x] Each `stk proxy` invocation is recorded to a local usage-history store: `~/.cache/stk/history.log` (respecting `XDG_CACHE_HOME`), one tab-separated, field-escaped line per entry (timestamp, verb, command, args) — `stk gain` (ticket 13) will read this.
- [x] No usage/tracking summary is printed to stdout or stderr — `proxy` is silent about the fact that it's tracking.
- [x] A golden-comparison test confirms `stk proxy` output matches real `rtk proxy` for the `echo_hello` fixture, plus a second golden test confirming the `-c`-is-not-a-flag behavior matches real `rtk` too (skipped gracefully without `rtk`).

**Implementation notes:**
- `cli::run`'s seam gained a `history: &dyn HistoryStore` parameter (same pattern as `CommandExecutor`), with `FakeHistoryStore` for tests and `FileHistoryStore` for `main.rs`.
- Shared passthrough error-formatting (`no_command_error`, `exec_error`) and `ExitInfo::to_process_exit_code` extracted so `run`/`proxy` can't drift out of sync.
- Code review caught and fixed 8 issues: `run -c` was silently dropping shell positional args after the command string (now forwarded, verified end-to-end); the history log had no escaping for tabs/newlines in args (now backslash-escaped); the history write was two syscalls instead of one, non-atomic under concurrent proxy runs (now a single `write_all`); the cache-dir fallback used a cwd-relative path when `HOME` is unset (now `temp_dir()`-based, still absolute); a signal-forwarding registration race between spawning the child and installing the forwarder (now registers the handler *before* spawning, closing that window — the separate, already-documented post-`wait()` PID-reuse race remains, judged disproportionate to fully close); the `-c` divergence from `run` was asserted only by comment, not tested against real `rtk` (now empirically verified and golden-tested); `proxy`'s tests never covered a plain non-zero/non-signal exit code; and the golden test wasn't isolating the real `rtk` invocation's own `XDG_CACHE_HOME`.
