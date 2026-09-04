# 05: Fast path — `stk compile` unbounded (Janitor + Aggregator)

**What to build:** `stk compile` (given a command to wrap, or reading stdin when none is given) runs the Janitor + Aggregator fast path: ANSI escape codes stripped, redundant whitespace/excess blank lines normalized, known boilerplate filtered, and exactly-repeated lines collapsed into `[repeated Nx]`. No `Budget`/priority/omission-marker logic yet — this ticket proves the deterministic cleanup pipeline alone, before selection logic is layered on top.

**Blocked by:** 04

**Status:** ready-for-agent

- [ ] `stk compile <cmd>` and `cat file | stk compile` both work (executing a command vs. reading stdin when none is given).
- [ ] ANSI escape codes are stripped from the output.
- [ ] Redundant whitespace and excessive empty lines are normalized.
- [ ] A configurable set of known-boilerplate patterns is filtered out.
- [ ] Exactly-repeated consecutive lines are collapsed to a `<line> [repeated Nx]` form.
- [ ] Unreasonably long lines are safeguarded against (don't defeat compression on their own).
- [ ] Fixture-driven tests cover at least: an ANSI-heavy log, a repeated-retry log, and a log with long lines — asserting the cleaned-up shape, not exact byte match.
