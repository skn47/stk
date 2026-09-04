# 03: `stk proxy` — tracked passthrough

**What to build:** `stk proxy [ARGS...]` behaves exactly like `stk run` — byte-identical stdout/stderr/exit code — but additionally records usage silently (no stderr summary printed), so developers who want `stk` in front of every command without any behavior change get adoption data for free.

**Blocked by:** 02

**Status:** ready-for-agent

- [ ] `stk proxy <cmd> [args...]` produces byte-identical output/exit code to running the command directly, reusing the same passthrough/signal-forwarding/streaming behavior as `stk run`.
- [ ] Each `stk proxy` invocation is recorded to a local usage-history store (command, timestamp) — schema is this ticket's call, since `stk gain` will read it later.
- [ ] No usage/tracking summary is printed to stdout or stderr — `proxy` is silent about the fact that it's tracking.
- [ ] A golden-comparison test confirms `stk proxy` output matches real `rtk proxy` for at least one fixture (skipped gracefully without `rtk`).
