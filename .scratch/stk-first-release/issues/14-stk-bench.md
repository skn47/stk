# 14: `stk bench --against rtk` — golden-output comparison suite

**What to build:** `stk bench --against rtk` runs the fixture corpus through both the real installed `rtk` and `stk` for every aliased verb (`run`, `proxy`, `pipe`, `init`, `config`, `gain`, and specialist dispatch for `cargo`/`git`), asserting `stk`'s output is equivalent-or-better and that mandatory evidence always survives — formalizing the ad hoc golden-comparison checks built incrementally in tickets 02-13 into one command developers can run themselves.

**Blocked by:** 02, 03, 04, 07, 08, 11, 12, 13

**Status:** ready-for-agent

- [ ] `stk bench --against rtk` runs every fixture in the corpus through both binaries for each aliased verb and reports a pass/fail/diff summary.
- [ ] The command pins/reports the `rtk` version it compared against (currently v0.42.4), flagging if a different version is detected.
- [ ] For each fixture, a required-evidence check (the fixture's `must_preserve` list) fails the run if `stk`'s output drops something `rtk`'s doesn't.
- [ ] The command exits nonzero if any fixture regresses, zero otherwise, so it's usable as a CI gate.
- [ ] Gracefully reports (not crashes) when the real `rtk` binary isn't present in the environment.
