# stk

`stk` (Smart Token Killer) is a Rust CLI: a stateful context compiler for coding agents.

The authoritative design spec is `../stk_implementation_overview.md` (outside this repo, at
`/home/h/stk_implementation_overview.md`). Read it before implementing anything non-trivial;
its §1.1 Review Log records resolved design findings.

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
