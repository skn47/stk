# 04: `stk pipe` — stdin + named filter, buffered-output path

**What to build:** `stk pipe [-f FILTER] [--passthrough]` reads stdin, applies one named filter, and prints the filtered result — the lightweight Unix-pipe mode, distinct from `compile`'s full scoring/budgeting. Since `pipe` never spawns a child process, this ticket is where the buffered-output half of the process-wrapper contract (as opposed to `run`/`proxy`'s live-streaming half) gets built, including graceful handling of a broken pipe on `stk`'s own stdout.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] `stk pipe -f FILTER` reads all of stdin, applies the named filter, and prints the result to stdout.
- [ ] `--passthrough` prints the input unmodified (a no-op baseline / testing aid).
- [ ] An unrecognized `-f FILTER` name fails with a clear error rather than silently passing through or crashing.
- [ ] If `stk pipe`'s stdout is closed early downstream (e.g. piped into `head`), `stk` exits gracefully rather than crashing or hanging (EPIPE handled).
- [ ] A golden-comparison test confirms `stk pipe` output matches real `rtk pipe` for at least one filter on at least one fixture (skipped gracefully without `rtk`).
