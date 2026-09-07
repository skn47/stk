# 01: `stk rewrite` + PreToolUse hook — auto-compress raw cargo/git commands

**What to build:** A `stk rewrite "<command string>"` verb that safely rewrites a raw `cargo`/`git`
invocation to its `stk`-prefixed equivalent (or refuses, never guessing), plus a
`PreToolUse:Bash` Claude Code hook (`.claude/hooks/stk-rewrite.sh`) that calls it and auto-applies
the rewrite, so an agent working in this repo gets stk's compression without remembering the
`stk ` prefix itself.

**Blocked by:** None (can start immediately)

**Status:** registered, pending live verification

- [x] `stk rewrite "cargo <anything>"` and `stk rewrite "git <anything>"` print `stk ` + the
      original string unchanged, exit 0 — including a chained form like `"cargo build && cargo test"`.
- [x] `stk rewrite` refuses (exit 1, nothing on stdout) for: empty input, a string already starting
      with `stk` as its own word, a string containing `<<` anywhere, and any command whose first
      word isn't exactly `cargo` or `git`.
- [x] The hook script reads `.tool_input.command` from stdin JSON, calls `stk rewrite`, and on exit
      0 emits `hookSpecificOutput` with `permissionDecision: "allow"` and the rewritten command
      substituted into `updatedInput`; on exit 1 (or `stk`/`jq` missing) it exits 0 with no output.
- [x] An opt-in audit log (`STK_HOOK_AUDIT=1`, overridable directory via `STK_AUDIT_DIR`) records
      `timestamp | original | rewritten` for each applied rewrite.
- [x] Registered in `.claude/settings.json`'s `hooks.PreToolUse` (matcher `Bash`), alongside the
      existing `PostToolUse` fmt hook. Claude Code's auto-mode classifier initially declined this
      edit as a self-modifying, permission-affecting change and asked for direct human action; the
      user then explicitly asked for it, and it was applied.
- [ ] **Not yet proven live**: a real `cargo check` Bash call in this session still ran unmodified
      after registration, even after fixing a real bug the attempt surfaced (the hook's
      `command -v stk` guard silently failed because hook subprocesses run with a minimal PATH that
      excludes `~/.cargo/bin` — fixed by exporting it at the top of the script). Claude Code's docs
      state settings-file hook edits are picked up by a file watcher without a restart, but that
      didn't happen in this session across three retries. Filed as product feedback; needs
      verification in a fresh Claude Code session before this can be marked fully done.

**Implementation notes:**
- `src/verbs/rewrite.rs` delegates to `specialists::lookup` (the existing registry in
  `src/specialists/mod.rs`) instead of a second hardcoded `"cargo"/"git"` list, so a future
  specialist is automatically rewrite-eligible with no second place to update.
- Code review caught and fixed 4 issues: two genuinely dead/redundant guard branches
  (an explicit empty-input check and an explicit already-"stk" check, both already implied by the
  surrounding control flow -- removing them changed no behavior, confirmed by the full existing
  test suite still passing); the hook's `set -euo pipefail` would have killed the script with an
  uncontrolled nonzero exit on malformed stdin JSON instead of the intended "pass through
  unchanged" -- fixed with an explicit `|| exit 0`; and the audit log didn't escape a literal `|`
  in the logged command, corrupting the pipe-delimited format -- fixed and verified with a
  `cargo test -- | tee out.log`-shaped input. `tests/rewrite_verb.rs` was trimmed from 5 tests
  re-checking pure-function outcomes already covered by `rewrite.rs`'s own unit tests down to one
  wiring smoke test, per this repo's stated convention that integration tests are for behavior
  that needs OS-level verification, not a second copy of unit coverage.
- One finding was raised but deliberately not changed: `dispatch` writes nothing to stderr on a
  "no rewrite applies" exit 1. That outcome is a normal, frequent result (like `grep`'s own exit 1
  for "no match"), not a failure -- and the hook script explicitly discards `stk rewrite`'s stderr
  (`2>/dev/null`) regardless, so a message there would only ever be seen by someone invoking
  `stk rewrite` by hand, not the hook.
