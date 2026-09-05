# 12: `stk init` — shell/agent integration install

**What to build:** `stk init [--global|-g] [--codex]` installs shell/agent integration idempotently, reporting exactly which files it changed. `--codex` writes ordinary project config (matching the real `rtk init --codex`'s install-time-only behavior — no runtime Codex-detection).

**Blocked by:** 01

**Status:** done

- [x] `stk init` (no flags) installs local/project-level integration and prints exactly which file(s) it created or modified.
- [x] `stk init --global`/`-g` installs to the global config location instead.
- [x] `stk init --codex` targets Codex CLI integration, writing to the same project config `stk config` (ticket 11) reads — no separate runtime-detection mechanism.
- [x] Running `stk init` (with the same flags) twice in a row is idempotent: the second run reports no changes rather than duplicating or corrupting anything.
- [x] Tests cover: first run reports changed files, second run reports none, `--global` targets a different location than the default.

**Implementation notes:**
- Plain `stk init` writes/updates a marked block (`<!-- stk-instructions v1 -->` ... `<!-- /stk-instructions -->`) in the project's `CLAUDE.md`, appending it to any existing file rather than clobbering prior content. `--global`/`-g` targets `$HOME/.claude/CLAUDE.md` instead. `--codex` is a separate branch entirely: it writes the project's `.stk/config.toml` (the same file `stk config` reads), matching the real `rtk init --codex`'s install-time-only, no-runtime-detection behavior confirmed against the installed binary. The three targets are modeled as one enum (`Target::{ProjectClaudeMd, GlobalClaudeMd, ProjectConfig}`) produced by a single parse step, so the dispatch match has no way to represent the invalid `--codex --global` combination once parsing succeeds.
- `.stk/config.toml` creation is shared with ticket 11 via a new `config::settings::create_default_file(path)` helper (atomic `create_new`, returns whether it created or found-existing) -- both `stk init --codex` and `stk config --create` now call the same function instead of duplicating the atomic-create logic.
- Code review caught and fixed 5 issues: the marker-block matcher paired the first `<!-- stk-instructions v1 -->` found with the *next* `<!-- /stk-instructions -->` found anywhere after it, so an orphaned/malformed start marker (e.g. from a bad merge) would silently absorb and delete real content up to an unrelated end marker on a second run -- reproduced exactly as described and fixed by refusing (not guessing) whenever the marker counts aren't a clean 0-or-1 pair, asking the user to resolve manually; a doc comment referenced a ticket number, against this repo's own comment convention; the `.stk/config.toml` atomic-create logic was duplicated between this ticket and ticket 11's `stk config --create` -- extracted to the shared helper above; the two mutually-exclusive-target bools plus a bolted-on rejection check were replaced with the single `Target` enum described above; and the CLAUDE.md write used a plain `read_to_string` + `fs::write` instead of an atomic write -- switched to write-to-temp-then-`rename` so an interrupted write can never leave a torn/partial file (a full concurrent-writer lock was judged out of scope for what this ticket needs).
- A regression fix surfaced during the marker-matching fix: consuming a trailing newline after a found end-marker span (needed so re-rendering doesn't double up blank lines on repeated `stk init` runs) had to be re-added to the new stricter matcher, since the first version of the fix dropped it and made every second run report "Updated" instead of "No changes needed" -- caught by the full test suite, not just the targeted new tests.
