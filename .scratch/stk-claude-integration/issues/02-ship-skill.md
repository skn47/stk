# 02: `ship` skill — release checklist reflecting what stk has today

**What to build:** A project-scoped Claude Code skill (`.claude/skills/ship/SKILL.md`) documenting
stk's release checklist: pre-release quality gate, semantic-versioning guidance using stk's own
verb surface, a manually-maintained changelog convention, the tag/push sequence, and a rollback
plan — each scoped to infrastructure stk actually has, with gaps (no CI, no crates.io metadata)
noted explicitly rather than assumed away.

**Blocked by:** None (can start immediately; no code dependency on ticket 01)

**Status:** done

- [x] Pre-release checklist: `cargo fmt --all --check`, `cargo clippy --all-targets --all-features`,
      `cargo test`, `stk bench --against rtk`, clean `git status`.
- [x] Version-bump guidance: `Cargo.toml`'s `version` field, semantic-versioning examples using
      stk's own verbs (e.g. a new verb is a MINOR bump, a specialist bug fix is a PATCH bump).
- [x] Changelog convention: a manually-maintained `CHANGELOG.md` (no release-please assumed), with
      a one-line note that automation is a future option, not a current fact.
- [x] Tag + push sequence (`git tag -a vX.Y.Z`, `git push origin main && git push origin vX.Y.Z`)
      that stops there — no GitHub Actions / `gh release view` steps, since no workflow exists yet;
      noted as "add once CI exists."
- [x] Rollback plan (revert commit, delete-and-recreate tag) with no `cargo yank` step, since
      `Cargo.toml` has no publish metadata and stk isn't on crates.io.
- [x] A `cargo audit` dependency-scan step as part of the pre-release checklist.
- [x] Verified by hand-walking the checklist once against the real current repo state (no actual
      tag/push) and confirming every command in it runs successfully as written.

**Implementation notes:**
- `.claude/skills/ship/SKILL.md`, project-scoped, `disable-model-invocation: true` (a release is
  always a deliberate human-initiated action, never something to auto-fire) with a plain
  human-facing description, following the `writing-for-agents` skill's invocation guidance.
- Hand-walked every command in the checklist against this repo's real current state: `cargo fmt
  --all --check`, `cargo clippy --all-targets --all-features`, `cargo test` (163 lib tests + all
  integration suites), and `stk bench --against rtk` (10/10) all passed; `git status` showed the
  expected in-progress changes from ticket 01, not a stray failure. `cargo-audit` wasn't installed
  yet -- installed it (`cargo install cargo-audit`) and ran `cargo audit`, which completed with no
  advisories found against the current `Cargo.lock` (27 crate dependencies), confirming that step
  is real and correct as written, not just plausible-looking prose.
