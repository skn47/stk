# 14: `stk bench --against rtk` — golden-output comparison suite

**What to build:** `stk bench --against rtk` runs the fixture corpus through both the real installed `rtk` and `stk` for every aliased verb (`run`, `proxy`, `pipe`, `init`, `config`, `gain`, and specialist dispatch for `cargo`/`git`), asserting `stk`'s output is equivalent-or-better and that mandatory evidence always survives — formalizing the ad hoc golden-comparison checks built incrementally in tickets 02-13 into one command developers can run themselves.

**Blocked by:** 02, 03, 04, 07, 08, 11, 12, 13

**Status:** done

- [x] `stk bench --against rtk` runs every fixture in the corpus through both binaries for each aliased verb and reports a pass/fail/diff summary.
- [x] The command pins/reports the `rtk` version it compared against (currently v0.42.4), flagging if a different version is detected.
- [x] For each fixture, a required-evidence check (the fixture's `must_preserve` list) fails the run if `stk`'s output drops something `rtk`'s doesn't.
- [x] The command exits nonzero if any fixture regresses, zero otherwise, so it's usable as a CI gate.
- [x] Gracefully reports (not crashes) when the real `rtk` binary isn't present in the environment.

**Implementation notes:**
- Ten cases cover the eight aliased verbs: `run`/`proxy` (exact stdout+exit-code match against `tests/fixtures/echo_hello.toml`'s real `command`, read via `toml::Table` rather than the dev-only derived `Fixture` struct, so production stays free of serde's derive machinery while still using the real corpus instead of retyped literals); `pipe` (exact match, passthrough and default); Cargo/Git specialist dispatch (required-evidence checks against the checked-in `cargo_projects/type_error` fixture and a dynamically-built temp git repo, matching each specialist's own ticket); `init`/`config`/`gain` (a smoke check -- both binaries must exit 0 on the same invocation, since these three deliberately don't produce byte-identical output to `rtk` by design, unlike the others).
- The other four TOML fixtures (`ansi_heavy`/`long_lines`/`p0_shrink`/`repeated_retries`) are deliberately not run by `stk bench`: they're `stk compile`-only regression fixtures, and `compile` has no `rtk` equivalent to compare against -- there's nothing for a "golden comparison" to mean there.
- `rtk`'s availability check moved into the library itself (`src/rtk.rs`) as the single source of truth for "is the real `rtk` installed," used by both `stk bench` and every golden-comparison test's `tests/support/rtk.rs` (now a thin wrapper, not a second copy of the same probe).
- A version mismatch against the pinned `0.42.4` warns but doesn't fail the run; `rtk` not being installed at all exits 0 with a message, so this never becomes an accidental hard dependency for environments (including CI) that don't have it.
- Code review caught and fixed 4 issues: `init`/`config`/`gain`'s smoke checks only isolated `XDG_CONFIG_HOME`/`XDG_CACHE_HOME` for the `stk` side, so a real, possibly-malformed `XDG_CONFIG_HOME` in the outer environment leaked into the `rtk` subprocess and could produce a spurious `[FAIL]` unrelated to any real `stk` regression -- reproduced exactly as described and fixed by isolating both sides symmetrically; the `run`/`proxy` cases hand-retyped `echo_hello.toml`'s data instead of reading the fixture -- fixed as described above; a duplicated `rtk`-availability probe -- extracted to `src/rtk.rs`; and two comments referencing ticket numbers / exceeding the 3-line cap.
