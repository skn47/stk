# 08: Git `Specialist`

**What to build:** `stk git <subcommand> [args...]` dispatches to the Git `Specialist`, which parses Git's raw output into `Chunk`s recognizing modified/staged files, conflict markers, diff hunks, rename information, and branch divergence — then reuses ticket 06's budgeted selection/rendering.

**Blocked by:** 06

**Status:** done

- [x] `stk git status` / `stk git diff` (and other Git subcommands) dispatch to the Git `Specialist` via `specialists::lookup`, alongside `cargo` (ticket 07).
- [x] Modified/staged files (both porcelain `M `/` M`/`MM`/`A `/`R `/`??` and human-readable `modified:`/`renamed:`/etc.) and branch-divergence info (`Your branch is ahead of...`/`...have diverged...`) are extracted as `P0` `Chunk`s.
- [x] Conflict markers (`<<<<<<<`/`=======`/`>>>>>>>`), `diff --git` headers, and hunk (`@@`) headers are all `P0` — the header stays with its hunks under most (not pathologically tight) budgets, since first release has no dependency-aware selection (`Chunk.relationships` is still unused, as documented) to *guarantee* it at the very tightest budgets — an already-flagged, deferred limitation of independent priority+recency scoring, not new to this ticket.
- [x] Rename information (`similarity index`/`rename from`/`rename to`, porcelain `R`) is `P0`, distinct from an unrelated delete+add.
- [x] Fixture-driven tests cover a multi-hunk-plus-rename diff and a real, currently-unresolved merge conflict, built dynamically in a tempdir per test (a nested `.git` can't be checked in as a fixture the way the Cargo projects were).
- [x] Golden-comparison tests confirm `stk git status`/`stk git diff` retain the same mandatory evidence (renamed file, modified file, conflicted path) real `rtk git status`/`rtk git diff` do (skipped gracefully without `rtk`).

**Implementation notes:**
- The classifier's rules were derived from real `git status`/`git diff`/`git merge` output captured against a deliberately modified/renamed/conflicted throwaway repo, not assumption.
- Code review caught and fixed 2 issues: the porcelain-status parser was called on the *trimmed* line, which deletes the leading-space status column that marks an unstaged-only change (e.g. `" M foo.rs"` becomes `"M foo.rs"`, silently misparsed and demoted to generic P3 -- reproduced, fixed by parsing the untrimmed line, regression-tested); and the human-readable conflict-status list was missing `"both deleted:"`, one of git's real conflict labels.
