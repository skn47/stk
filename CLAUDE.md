# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

**stk (Smart Token Killer)** is a Rust CLI that filters and compresses command output before
it reaches an agent's context, within an explicit token budget. It's a from-scratch alternative
to `rtk`, aiming to fix the class of bug `rtk` has (stateless compression, no hard budget
guarantee) rather than patch around it.

### Name Collision Warning

`crates.io/crates/stk` is an unrelated, pre-existing package (a scripting language, last released
2020). `cargo install stk` installs the wrong binary. Until this project claims its own crate
name or publishes under a different one, always install from source:

```bash
cargo install --path .
stk gain   # sanity check: should print a savings summary, not "command not found"
```

## Development Commands

### Build & Run
```bash
cargo build              # raw
stk cargo build          # preferred
cargo build --release
cargo run -- <command>
cargo install --path .
```

### Testing
```bash
cargo test                # all tests
stk cargo test            # preferred
cargo test <test_name>
cargo test -- --nocapture
```

### Linting & Quality
```bash
cargo check
cargo fmt
cargo clippy --all-targets --all-features
stk cargo clippy --all-targets --all-features   # preferred
```

### Pre-commit Gate
```bash
cargo fmt --all --check && cargo clippy --all-targets --all-features && cargo test
```
Matches `.github/workflows/ci.yml` exactly -- if this fails locally, CI fails too.

`.claude/hooks/stk-rewrite.sh` (registered as a `PreToolUse` hook in `.claude/settings.json`)
already rewrites plain `cargo`/`git` Bash calls to their `stk` equivalent automatically, so typing
the raw form is fine day-to-day.

## Coding Rules

- **One seam**: extend `cli::run` rather than adding a second dispatch path.
  `CommandExecutor`/`HistoryStore` are the swappable dependencies -- `Real*` for production,
  `Fake*` for tests.
- Errors go to stderr as `stk: <message>` with a nonzero exit code -- no `panic!`/`unwrap` in
  dispatch paths.
- `ExitInfo::to_process_exit_code()` is the one place child exit/signal info becomes `stk`'s own
  exit code (128+signal on signal death) -- reuse it, don't reimplement.
- Comments: sparing. Default to none; when one's needed, 1 line (3 max), explaining WHY not WHAT,
  never referencing ticket/phase numbers.
- Commits: Conventional Commits (`feat`, `fix`, `docs`, `refactor`, `test`, `chore`), no
  ticket-ID prefixes.

## Testing

- **Unit tests**: inline per module (`#[cfg(test)] mod tests`), exercise dispatch through the
  fakes -- no real subprocesses.
- **Integration tests** (`tests/*.rs`): spawn the compiled binary (`env!("CARGO_BIN_EXE_stk")`)
  only for OS-level behavior (signals, byte-identical passthrough, EPIPE). Filesystem filters
  (`read`/`grep`/`find`) test through `cli::run` in-process against real temp-directory files
  instead, since they never touch `CommandExecutor`.
- **Fixture corpus**: `tests/fixtures/*.toml` (`command`, `must_preserve`, `may_remove`), loaded
  via `tests/support/fixture.rs`.

## Build Verification (Mandatory)

After any Rust file edit, run the full gate before committing:

```bash
cargo fmt --all --check && cargo clippy --all-targets --all-features && cargo test
```

Zero clippy tolerance -- fix every warning before moving on.

## Working Agreements

- Confirm `pwd` and `git branch` before file operations if there's any doubt which project or
  branch you're in.
- Don't chase external API/documentation rabbit holes on your own judgment -- if verifying
  something would take more than 3-4 exploratory commands, stop and ask first.
- For a numbered/multi-step plan: execute in order, don't skip or reorder without flagging it,
  and track 3+ step plans with TaskCreate/TaskUpdate.
- `.scratch/` is gitignored (local scratch space only) -- `.scratch/stk-first-release/spec.md`
  and `.scratch/stk-first-release/issues/` exist locally but not in a fresh clone.
