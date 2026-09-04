# 12: `stk init` — shell/agent integration install

**What to build:** `stk init [--global|-g] [--codex]` installs shell/agent integration idempotently, reporting exactly which files it changed. `--codex` writes ordinary project config (matching the real `rtk init --codex`'s install-time-only behavior — no runtime Codex-detection).

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] `stk init` (no flags) installs local/project-level integration and prints exactly which file(s) it created or modified.
- [ ] `stk init --global`/`-g` installs to the global config location instead.
- [ ] `stk init --codex` targets Codex CLI integration, writing to the same project config `stk config` (ticket 11) reads — no separate runtime-detection mechanism.
- [ ] Running `stk init` (with the same flags) twice in a row is idempotent: the second run reports no changes rather than duplicating or corrupting anything.
- [ ] Tests cover: first run reports changed files, second run reports none, `--global` targets a different location than the default.
