# stk

`stk` is a Rust CLI: a stateful context compiler for coding agents — it turns noisy
shell/tool output into the smallest high-value context an agent needs, within an
explicit token budget, improving on stateless compressors like `rtk`.

## Where things live

- `.scratch/stk-first-release/spec.md` — the first-release spec (problem, solution, user stories, decisions)
- `.scratch/stk-first-release/issues/` — tracer-bullet tickets in dependency order, each with a `Status:` line
- `docs/adr/` — architecture decisions (e.g. why omission markers are `Chunk`s, why the token counter is a heuristic)
- `CONTEXT.md` — the domain glossary; use its terms, not synonyms

## Testing patterns

- **One seam**: `cli::run(args, stdin, stdout, stderr, executor, history) -> i32` is the
  only entry point every verb dispatches through. `CommandExecutor` and `HistoryStore`
  are the swappable dependencies — `Real*` for production (`main.rs`), `Fake*` for
  tests. Extend this seam rather than adding a second one.
- **Unit tests** live inline per module (`#[cfg(test)] mod tests`) and exercise dispatch
  logic through the fakes — no real subprocesses.
- **Integration tests** (`tests/*.rs`) spawn the actual compiled binary
  (`env!("CARGO_BIN_EXE_stk")`) when a behavior can only be verified at the OS level
  (signal forwarding, byte-identical passthrough, EPIPE handling) -- except the
  filesystem filters (`read`/`grep`/`find`), which never touch `CommandExecutor` at all
  and so test through `cli::run` in-process against real temp-directory files instead.
- **Golden-comparison tests** run the same invocation through `stk` and the real
  installed `rtk` binary, asserting equivalence; skip (don't fail) when `rtk` isn't
  installed (`tests/support/rtk.rs`). Verify claims about `rtk`'s actual behavior against
  the real binary before coding to them — don't assume from memory or its `--help` alone.
- **Fixture corpus**: `tests/fixtures/*.toml` (`command`, `must_preserve`, `may_remove`),
  loaded via `tests/support/fixture.rs`.

## Rust patterns

- Verbs live one-per-file under `src/verbs/`, each exposing a `dispatch(args, ...) -> i32`.
- Errors go to stderr as `stk: <message>` with a nonzero exit code — no `panic!`/`unwrap`
  in dispatch paths.
- `ExitInfo::to_process_exit_code()` is the one place child exit/signal info becomes
  `stk`'s own exit code (128+signal on signal death) — reuse it, don't reimplement.
- Comments: sparing. Default to none; when one's needed, 1 line (3 max), explaining WHY
  not WHAT, never referencing ticket/phase numbers.
- Commits: Conventional Commits (`feat`, `fix`, `docs`, `refactor`, `test`, `chore`), no
  ticket-ID prefixes.

## Agent skills

### Issue tracker

Issues and specs live as markdown files under `.scratch/<feature-slug>/` in this repo.
See `docs/agents/issue-tracker.md`.

### Triage labels

The five canonical triage roles, using their default label strings.
See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` and `docs/adr/` at the repo root.
See `docs/agents/domain.md`.
