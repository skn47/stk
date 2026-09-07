# Research: GitHub Actions CI setup for `stk`

Research only — no `.github/workflows/*.yml` written. Closes the "no CI" gap noted in
`.claude/skills/ship/SKILL.md` and the recent release-readiness audit.

## 1. Toolchain action

`actions-rs/toolchain` is **archived**: "This repository was archived by the owner on Oct 13,
2023. It is now read-only." (github.com/actions-rs/toolchain, fetched 2026-09-07). Do not use it.

The current recommended replacement is **`dtolnay/rust-toolchain`** (github.com/dtolnay/rust-toolchain).
Minimal usage:

```yaml
- uses: actions/checkout@v7
- uses: dtolnay/rust-toolchain@stable
- run: cargo test --all-features
```

Version pinning is via the action's `@rev` ref (`@stable`, `@nightly`, `@1.89.0`, or an
expression like "stable minus 8 releases"), not an `with: toolchain:` input on a maintained
long-lived action the way `actions-rs/toolchain` worked.

## 2. Jobs to mirror — this repo's own existing gate, not a new one

`.claude/skills/ship/SKILL.md` (§1, "Quality gate") and `CLAUDE.md`'s testing-pattern section
already define this project's canonical check set:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features
cargo test
stk bench --against rtk
```

A CI workflow should run exactly these — not invent a different lint/format policy — so that
"green in CI" and "green per the ship skill" stay the same statement. `cargo test` alone
covers both the inline unit tests (`#[cfg(test)] mod tests`) and the `tests/*.rs` integration
suite per CLAUDE.md's testing-patterns section; no separate invocation is needed for either.

## 3. Caching

**`Swatinem/rust-cache@v2`** (github.com/Swatinem/rust-cache) is the standard community action
for this. It must run *after* the toolchain is installed, since it uses the resolved `rustc`
version as part of its cache key, along with a hash of `Cargo.lock`/`Cargo.toml` and any
`rust-toolchain`/`.cargo/config.toml` files. Minimal usage:

```yaml
- uses: dtolnay/rust-toolchain@stable
- uses: Swatinem/rust-cache@v2
```

The project's own README notes it builds on GitHub's upstream `actions/cache`, so it inherits
that service's storage limits rather than introducing a separate caching backend.

## 4. Release-binary builds — scope this out for now (open question, not a decision)

`Cargo.toml` currently has no `description`/`license`/`repository`/`keywords` (confirmed:
`cat Cargo.toml` shows only `[package] name/version/edition` and `[[bin]]`/`[lib]` sections —
no publish metadata), and the project isn't on crates.io yet. A cross-platform release-binary
matrix job (building `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, etc., the way
`rtk-ai/rtk`'s own releases do — see §6) is meaningful once there's a release process to attach
binaries to, but building it before that process exists is scope creep on the CI ticket. Flagging
this as a **scoping question for whoever picks up the CI ticket**, not deciding it here: a basic
test/lint/fmt workflow is the immediate, load-bearing need; release-artifact builds are a
follow-on once `cargo publish` metadata and a tagged-release workflow exist.

## 5. Triggers

No single official GitHub doc mandates one convention, but the overwhelmingly standard pattern
for a library/CLI repo — including `dtolnay/rust-toolchain`'s own example workflow shown in §1 —
is:

```yaml
on: [push, pull_request]
```

i.e., run on every push to `main` and on every pull request. This matches what
`dtolnay/rust-toolchain`'s own README example workflow uses verbatim.

## 6. `stk bench --against rtk` in CI — tradeoff, not a decision

`stk bench` needs a real `rtk` binary on `PATH`; it currently **skips gracefully (exit 0)** when
`rtk` isn't installed (per `.claude/skills/ship/SKILL.md` §1 and `src/verbs/bench.rs`'s
`CaseOutcome::Skipped` variant). Two real options:

**Option A — install rtk in CI too.** `rtk-ai/rtk` publishes prebuilt binaries per GitHub
release, confirmed via the GitHub API (`gh api repos/rtk-ai/rtk/releases`), including
`rtk-x86_64-unknown-linux-musl.tar.gz` — installable on a standard `ubuntu-latest` runner with a
`curl`+`tar` step, no build-from-source needed. Confirmed the pinned version stk's golden tests
were last verified against, `v0.42.4` (see `src/verbs/bench.rs`'s `PINNED_RTK_VERSION`
constant), still has a matching GitHub release with that Linux asset
(`gh api repos/rtk-ai/rtk/releases/tags/v0.42.4`) — so pinning CI to that exact release is
possible today. This gets the real golden-comparison signal in CI, matching what the `ship`
skill's own quality gate demands locally, and catches the case where `stk`'s bench suite
silently "passes" in CI only because it's skipping.

**Option B — leave it local-only.** Since `stk bench` already degrades gracefully without `rtk`,
CI can simply run it as-is and accept "0 passed / 0 failed, all skipped" as a passing CI result,
relying on humans to run the real comparison locally (as the `ship` skill's checklist already
requires before a release). Simpler CI, but means a broken `stk`-vs-`rtk` equivalence could ship
without CI ever catching it — CI would need a way to detect the skip-only condition and fail if
that's not acceptable.

Option A costs a few CI-workflow lines and one version-pin decision (track `rtk`'s releases and
bump `PINNED_RTK_VERSION` + the CI install step together); Option B costs nothing but leaves the
project's stated core differentiator (matching/beating `rtk`) unverified by CI. Left as a real
choice for whoever implements the workflow — not decided here.

## Sources

- github.com/dtolnay/rust-toolchain (fetched 2026-09-07): recommended toolchain-install action, usage example, version-pin mechanism.
- github.com/actions-rs/toolchain (fetched 2026-09-07): archived Oct 13, 2023 — do not use.
- github.com/Swatinem/rust-cache (fetched 2026-09-07): caching action, cache-key strategy, usage example.
- `gh api repos/rtk-ai/rtk/releases` (queried 2026-09-07): confirmed per-release prebuilt binary assets including a Linux musl static tarball.
- `gh api repos/rtk-ai/rtk/releases/tags/v0.42.4` (queried 2026-09-07): confirmed the pinned version has a matching release with the same asset set.
- `/home/h/stk/.claude/skills/ship/SKILL.md` §1: this repo's own documented quality-gate commands.
- `/home/h/stk/CLAUDE.md` "Testing patterns" section: unit vs. integration vs. golden-comparison test layout.
- `/home/h/stk/src/verbs/bench.rs`: `PINNED_RTK_VERSION` constant and `CaseOutcome::Skipped` graceful-skip behavior.
- `/home/h/stk/Cargo.toml`: confirmed absence of `description`/`license`/`repository`/`keywords`.
