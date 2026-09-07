---
name: ship
description: Cut a stk release - quality gate, version bump, changelog, git tag, push.
disable-model-invocation: true
---

# Ship a stk release

Reflects what this repo actually has today: CI builds and tests stk itself only (no release
workflow, no crates.io metadata, no changelog automation). Gaps are called out as gaps, not
silently worked around.

## 1. Quality gate

`.github/workflows/ci.yml` runs fmt/clippy/build/test on every push/PR, so a green PR has
already cleared those -- but confirm locally before tagging, since CI green is as of the last
push, not as of your current working tree. `stk bench --against rtk` is deliberately **not**
part of CI (it depends on installing a third-party binary CI has no business installing); run it
locally as part of this gate, and keep the README's comparison numbers against `rtk` and similar
tools (e.g. `tare`) current instead:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features
cargo test
stk bench --against rtk
```

`stk bench` needs the real `rtk` installed to compare against; it skips gracefully (exit 0) if
`rtk` isn't present, but a `[FAIL]` line anywhere in its output is a blocker.

```bash
git status   # must be clean -- nothing to commit
```

## 2. Choose the version bump

Semantic versioning (MAJOR.MINOR.PATCH), examples from stk's own history:

- **MINOR** (0.X.0): a new verb (`stk bench`, `stk gain`) or new user-facing behavior.
- **PATCH** (0.0.X): a bug fix (a Specialist misclassifying a line, a budget edge case).
- **MAJOR** (X.0.0): a breaking CLI change (removing/renaming a verb or flag). Rare pre-1.0.

Edit `Cargo.toml`'s `version` field. `Cargo.lock` updates on the next `cargo build`.

## 3. Update the changelog

`CHANGELOG.md` is manually maintained -- there is no release-please or other automation wired up
for this yet. If the file doesn't exist, create it first with a
[Keep a Changelog](https://keepachangelog.com/) header:

```markdown
# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/), and this project follows
[Semantic Versioning](https://semver.org/).
```

Then add (or promote an existing `## [Unreleased]` section into) a dated release section:

```markdown
## [0.2.0] - 2026-09-05

### Added
- `stk bench --against rtk`: golden-comparison suite for every aliased verb.

### Fixed
- Git specialist: rename info no longer dropped under a tight budget.
```

## 4. Commit, tag, push

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore(release): bump version to vX.Y.Z"

git tag -a vX.Y.Z -m "Release vX.Y.Z

<paste the changelog section's Added/Fixed/Changed bullets here>"

git push origin main
git push origin vX.Y.Z
```

Stop here. There is no GitHub Actions release workflow yet, so nothing builds binaries or
publishes a GitHub Release automatically -- add those steps to this skill once that CI exists,
rather than describing a trigger that doesn't fire anything today.

## 5. Dependency security check

Run this before releasing, not after:

```bash
cargo install cargo-audit   # once, if not already installed
cargo audit
```

If a vulnerability is found, update the flagged dependency and re-run the full quality gate (step
1) before proceeding.

## Rollback

**Preferred — patch release**: branch, fix, bump PATCH, repeat steps 1-5.

**Last resort — revert the tag**:

```bash
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
git revert HEAD
git push origin main
```

No `cargo yank` step: `Cargo.toml` has no `description`/`license`/`repository` metadata yet and
stk isn't published to crates.io, so there's nothing to yank. Add this step if that changes.
