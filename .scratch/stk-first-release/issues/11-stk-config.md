# 11: `stk config` — show/create + precedence chain

**What to build:** `stk config` / `stk config --create` shows or creates the STK config file. Configuration precedence is `CLI flags > environment variables > repository/project config file > global config file`, with no separate runtime "Codex-detection" layer — `stk init --codex` (ticket 12) simply writes to the same project config this ticket reads.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] `stk config` prints the effective, resolved configuration (after precedence is applied).
- [ ] `stk config --create` writes a config file with sensible defaults if none exists, and does not clobber an existing one.
- [ ] Precedence is enforced in the documented order: CLI flags win over env vars, which win over repository/project config, which wins over global config.
- [ ] The `intent`, `session_memory`, and `session_ttl_minutes` config keys are accepted and parsed (schema forward-compatibility) but have no behavioral effect this release.
- [ ] Tests cover at least one case per precedence level overriding the one below it.
