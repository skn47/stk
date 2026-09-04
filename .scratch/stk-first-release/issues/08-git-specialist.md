# 08: Git `Specialist`

**What to build:** `stk git <subcommand> [args...]` dispatches to the Git `Specialist`, which parses Git's raw output into `Chunk`s recognizing modified/staged files, conflict markers, diff hunks, rename information, and branch divergence — then reuses ticket 06's budgeted selection/rendering.

**Blocked by:** 06

**Status:** ready-for-agent

- [ ] `stk git status` / `stk git diff` (and other Git subcommands) dispatch to the Git `Specialist` rather than the generic pipeline.
- [ ] Modified/staged files and branch-divergence info are extracted as `Chunk`s.
- [ ] Conflict markers and diff hunks are preserved with correct file association under `--budget`.
- [ ] Rename information is preserved rather than treated as an unrelated delete+add.
- [ ] Fixture-driven tests cover at least: a `git diff` with multiple hunks, and a merge-conflict fixture.
- [ ] A golden-comparison test confirms `stk git status`/`stk git diff` retains at least the mandatory evidence `rtk git` does, on the same fixture (skipped gracefully without `rtk`).
