## Problem Statement

Nothing today makes an agent working in this repo actually reach for `stk` instead of raw
`cargo`/`git` commands — it depends entirely on remembering to type the prefix. Separately, there
is no documented process for cutting a release of `stk` itself: no version-bump checklist, no
changelog convention, no tag/push sequence.

## Solution

Two small, independent additions to this repo's Claude Code integration, informed by reading
rtk's own `.claude/` directory (`github.com/rtk-ai/rtk/tree/develop/.claude`) for prior art:

1. A `stk rewrite` subcommand plus a `PreToolUse:Bash` hook that transparently rewrites a raw
   `cargo`/`git` command to its `stk`-prefixed (compressed) equivalent before it runs.
2. A project-scoped `ship` skill documenting the release checklist stk actually has today.

Neither is part of the tracked first-release spec (`.scratch/stk-first-release/`) — this is
Claude-Code tooling, not a shipped CLI feature for end users, though `stk rewrite` is a real verb
built with the same rigor (tests, fmt, clippy, code review) as any other.

## User Stories

1. As an agent working in this repo, I want a raw `cargo build`/`cargo test`/`git status`/etc. I
   type to be silently rewritten to its `stk`-prefixed equivalent, so that I get token-compressed
   output without having to remember the prefix myself.
2. As an agent, I want the rewrite to never touch a command that isn't safely rewritable (already
   `stk`-prefixed, not cargo/git, or a heredoc), so that it never corrupts or double-wraps a command.
3. As a maintainer, I want a documented, runnable release checklist reflecting what this repo
   actually has (no CI, no crates.io metadata, no changelog automation yet), so that cutting a
   release isn't reinvented from scratch each time and doesn't silently assume infrastructure that
   doesn't exist.

## Implementation Decisions

**Why cargo/git only, not read/grep/find/cat**: rtk's own filters are argument-compatible wrappers
around many real tools, so broad auto-rewrite is safe there. stk's native filters are narrower and
not flag-compatible with the real thing (`verbs/read.rs` takes only a bare path with no flags;
`verbs/grep.rs` is exactly `<pattern> <path>` positionally, no regex flags; `verbs/find.rs`'s
`--name` isn't real `find`'s `-name`). Rewriting `cat -n file` to `stk read -n file` would silently
misread `-n` as the path. Cargo/git specialist dispatch always executes the real binary with the
exact args given unchanged (`compression::execute_and_compress`) and only post-processes captured
output, so prepending `stk ` in front is always semantically safe, including a chained form like
`cargo build && cargo test` (the prefix only ever goes at the very front of the string).

**`stk rewrite` contract**: deliberately simpler than rtk's 4-way (allow/no-match/deny/ask) exit
code contract. stk has no per-command trust/deny config of its own; Claude Code's own permission
system evaluates trust for the rewritten command independently. `stk rewrite` only ever says
"here's a safe rewrite" (exit 0 + stdout) or "no rewrite" (exit 1).

**The hook sets `permissionDecision: "allow"`** on a successful rewrite, matching rtk's own hook.
This is a deliberate, narrow permission-model change (trusting "cargo/git, any args" wholesale) —
flagged explicitly in the ticket rather than left implicit, since it's exactly the kind of thing
worth a second look before merging.

**`ship` skill scope**: reflects what stk has today, not aspirational CI. No changelog automation
(manually maintained `CHANGELOG.md` instead), no GitHub Actions release trigger (none exists yet),
no `cargo yank` step (`Cargo.toml` has no publish metadata and isn't on crates.io). Each gap is
noted as a gap, not silently assumed away or invented.

## Testing Decisions

`stk rewrite` is a pure string transform (no I/O, no subprocess, no compression) — inline unit
tests cover each branch of its algorithm directly, no fakes needed. One integration test spawns the
real binary for the plain and chained (`&&`) cases, matching this repo's established convention of
spawning the real binary only when a behavior needs OS-level verification (here: matching how
`cli.rs`'s other simple verbs like `bench`/`init` are tested).

The `ship` skill is pure prompt content with no code — its "test" is a single hand-walk of the
checklist against this repo's real current state (no actual tag/push), confirming every command in
it is real and correct, not aspirational.

## Out of Scope

- The suggest-only tier for `cat`/`grep` (a soft system-message nudge without auto-rewriting) that
  rtk also has — a real follow-up, not built here, since the safe cargo/git case is where this
  session's actual friction has been.
- Any deny/ask tier inside `stk rewrite` itself.
- Adopting release-please, a GitHub Actions release workflow, or crates.io publish metadata.
- Leading env-var-assignment (`FOO=bar cargo build`) or `cd x &&`-prefix recognition in
  `stk rewrite` — a documented known limitation.

## Further Notes

Both tickets can be built in either order or in parallel — ticket 02 (the `ship` skill) has no
code dependency on ticket 01 (the rewrite hook).
