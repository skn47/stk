# 13: `stk gain` — savings summary/history

**What to build:** `stk gain [-p] [-H] [-g] [-q] [-t TIER] [-d|-w|-m|-a] [-f FORMAT] [-F] [--reset]` shows a token-savings summary/history, matching the documented subset of `rtk gain`'s flags. This requires wiring lightweight savings-recording into `compile`, the two `Specialist`s, and the filters delivered so far, so there's real data to summarize.

**Blocked by:** 06, 07, 08, 09, 10

**Status:** ready-for-agent

- [ ] `compile` (ticket 06), the Cargo/Git `Specialist`s (07/08), and the filters (09/10) each record their input/output token counts to a local savings-history store as they run.
- [ ] `stk gain` (no flags) prints an overall savings summary (input tokens, output tokens, reduction).
- [ ] History/tier/date-range flags (`-H`, `-t TIER`, `-d|-w|-m|-a`) filter or window the summary as documented.
- [ ] `-f FORMAT` changes output format; `--reset` clears recorded history.
- [ ] A golden-comparison test confirms `stk gain`'s output shape is compatible with the documented `rtk gain` flag subset (skipped gracefully without `rtk`).
