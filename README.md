<div align="center">

<img src="docs/assets/logo.svg" alt="stk" width="220">

**CLI proxy that reduces LLM token consumption by up to ~80% on common dev commands. Single Rust binary, zero dependencies.**

</div>

## What STK Does

STK (Smart Token Killer) filters and compresses outputs before they reach your LLM context. Single Rust binary, native Cargo/Git specialists plus a generic budgeted pipeline for everything else, <5ms overhead on typical git operations.

It's a from-scratch, improved alternative to [`rtk`](https://github.com/rtk-ai/rtk), tracked
against it as demonstrated by the numbers in [Performance](#performance) below.

## How Savings Work

STK cuts up to ~54-79% of raw command output on cases large enough to exceed its default 2000-token budget. That's what STK bench-style comparisons and the token-estimate table capture, and it is not the same as cutting your bill by that much.

## Why

Agent context windows fill up fast with raw tool output most of which is noise like passing test
output, unchanged diff context, or duplicate warnings.

`stk` prefixes your usual commands and
compresses their output before it reaches the agent without you having to change how you work.

## How It Works

```
Without stk:                                    With stk:

Claude  --git status-->  shell  -->  git         Claude  --git status-->  STK  -->  git
  ^                                    |            ^                     |          |
  |         full raw output            |            |  compact output     | filter   |
  +------------------------------------+            +------ (filtered) ---+----------+
```

Three strategies apply:

- **Priority-based selection** - Scores every line, drops the lowest-value ones first under budget pressure
- **Truncation** - Cuts long lines and over-budget content
- **Deduplication** - Aggregates repeated lines and boilerplate dividers into distinct components

**Does stk break your AI tool's prompt cache?** No. `stk` filters a command's output once, before the agent ever sees it. The result is just what gets sent, so your tool's own prompt cache works on it normally like any other output. Smaller outputs also mean cheaper cache writes and reads.
## Install

Not yet published to crates.io, build from source:

```
git clone https://github.com/skn47/stk.git
cd stk
cargo install --path .
```

This installs `stk` to `~/.cargo/bin/stk` (make sure that's on your `PATH`).

## Quick start

```
# Install for your AI tool
stk init -g --claude               # Claude Code
stk init -g --codex                # Codex (OpenAI)

(more coming)
```

## Commands

### Files

```bash
stk read file.rs                # Smart file reader (head/tail truncation, not a cat wrapper)
stk grep "pattern" path/        # Substring search
stk find path/ --name pattern   # Recursive listing (substring match, not a glob)
```

### Git

```bash
stk git status                  # Compact status
stk git diff                    # Condensed diff (chunks + renames kept, boilerplate dropped)
stk git <any other subcommand>  # Runs the real binary, compressed using the default filterer
```

### Cargo

```bash
stk cargo check                 # Errors/locations kept, build-phase noise dropped
stk cargo build --release       # Same, for build output
stk cargo test                  # Failures + assertion detail kept, passing tests deprioritized
stk cargo clippy                # Same compression as check/build
```

### Generic output & streams

```bash
stk compile <cmd>                   # Budgeted pipeline for any command with no specialist
some_noisy_command | stk compile    # Alternatively use with piped stdin
stk log [FILE]                      # Dedupe/filter a log stream (file or stdin)
stk err <cmd>                       # Hard filter: only errors/warnings survive
stk summary <cmd>                   # Heuristic: counts + first error only
stk diff [FILE|-]                   # Condense existing unified-diff text to hunk headers + changes
stk json FILE [--keys-only]         # Compact JSON, or structure-only
stk pipe [-f ansi] [--passthrough]  # Named stdin filter (only `ansi` exists today)
```

### Passthrough

```bash
stk run <cmd>                   # Raw passthroug
stk proxy <cmd>                 # Byte-identical passthrough, tracked for `stk gain`
```

### Config & analytics

```bash
stk config                      # Show the effective, precedence-resolved config
stk config --create             # Create the global config file with defaults
stk gain                        # Token-savings summary
stk gain -p                     # Scoped to the current project instead of global
stk gain -H                     # Per-command history
stk gain -d / -w / -m           # Daily / weekly / monthly breakdown
stk gain --reset                # Clear recorded history
```

## Global Flags

```bash
--budget N    # Set a token ceiling for commands
```

## Performance

Real numbers, measured locally against `rtk` v0.42.4, not estimates.

Measured on a 12th Gen
Intel Core i5-1240P running Ubuntu 24.04.3 LTS (x86_64), both binaries built in `--release` mode.

**Compression (estimated tokens, lower is better):**

| Case | raw | rtk | stk | rtk vs raw | stk vs raw |
|---|---:|---:|---:|---:|---:|
| `cargo check` (1 error) | 216 | 189 | 216 | -12.5% | 0.0% |
| `cargo build` (60 warnings) | 4238 | 4155 | **1953** | -2.0% | **-53.9%** |
| `git diff` (2 files) | 117 | 107 | 117 | -8.5% | 0.0% |
| `git diff` (25 files) | 8925 | 8104 | **1918** | -9.2% | **-78.5%** |

Token counts are estimates from stk's own counter (`ceil(max(chars/3, words*1.4))`), applied identically to the raw/rtk/stk output of each command. Not a real model tokenizer, but the same metric for all three columns.

<br>

**Speed (wall-clock, lower is better):**

| Case | raw | rtk | stk |
|---|---:|---:|---:|
| `git status` | 6ms | 22ms | 8ms |
| `git diff` (2 files) | 5ms | 26ms | 7ms |
| `git diff` (25 files) | 8ms | 28ms | 16ms |
| `cargo check` | 92ms | 100ms | 82ms |
| `cargo build` (60 warnings) | 29ms | 38ms | 34ms |

Each figure is a wall-clock average over 15-20 runs (`date +%s%N` before/after) with a
warm build cache. Additionally, `strace -c` on `git status` confirmed stk makes roughly half rtk's
`openat` syscalls, which lines up with the speedup on git operations.


## Roadmap

Ideas for future versions:

- **Session / temporal memory** - remember command history, error locations, and touched files
  across invocations sharing a repo root + cwd, so repeated commands can get smarter than a
  stateless compressor ever could.
- **Tree-sitter structural slicing** - for `stk read`/`stk grep`, return the enclosing function
  or type instead of raw line ranges.
- **Intent Engine (`--intent`)** - a task-purpose hint (`debug`, `docs`, `test`, `review`,
  `security`) that steers relevance scoring under budget pressure.
- **Explainability (`--explain`)** - show *why* a chunk was kept or dropped, not just the result.
- **Cross-file context graph** - follow references (a failing test to the function it tests) so
  budget is spent on the right file, not just the one that happened to error.
- **More specialists** - pytest, Node/npm, and other ecosystems beyond Cargo/Git.
- **PTY / interactive child support** - commands that expect a real terminal (interactive
  prompts, progress bars that redraw in place).
- **Shell-grammar-aware `stk rewrite`** - currently only rewrites when the literal first word is
  `cargo`/`git`; a leading env-var assignment (`FOO=bar cargo build`) or `cd x &&` prefix passes
  through unrewritten.
- **crates.io publishing** - `Cargo.toml` metadata (`description`/`license`/`repository`), a
  `CHANGELOG.md`, and a tagged-release GitHub Actions workflow that builds binaries.
