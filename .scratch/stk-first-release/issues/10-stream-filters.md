# 10: Stream filters — `stk log` / `stk err` / `stk summary` / `stk diff` / `stk json`

**What to build:** `stk log`, `stk err`, `stk summary`, `stk diff`, and `stk json` as native, `Budget`-aware filters over piped or command text, distinct from `rtk`'s tool-wrapping versions of the same names.

**Blocked by:** 06

**Status:** ready-for-agent

- [ ] Each of `log`, `err`, `summary`, `diff`, `json` accepts stdin (and/or a command to run, consistent with the other verbs' capture conventions) and renders `Budget`-bounded output.
- [ ] `err` isolates errors/warnings from mixed output; `summary` produces a heuristic condensed summary; `log` filters/deduplicates; `diff` condenses to changed lines/hunks; `json` compacts values (or keys-only, matching the documented flag).
- [ ] All five are stateless in this release (no `Session`-aware behavior).
- [ ] Fixture-driven tests cover at least one representative case per filter.
