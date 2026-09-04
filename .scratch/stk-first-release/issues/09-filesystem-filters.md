# 09: Filesystem filters — `stk read` / `stk grep` / `stk find`

**What to build:** `stk read <file>`, `stk grep ...`, and `stk find ...` as native, `Budget`-aware filters operating directly on the real filesystem (not wrapping `cat`/`rg`/`find` the way `rtk`'s versions do). Since these don't spawn a child process, they're tested against real temp-directory fixture files, not the `CommandExecutor` seam.

**Blocked by:** 06

**Status:** ready-for-agent

- [ ] `stk read <file> --budget N` renders a token-bounded view of the file using conservative text-based slicing (head/tail preservation, no Tree-sitter — that's out of scope this release).
- [ ] `stk grep <pattern> <path>` and `stk find <path> [flags]` are native implementations (not subprocess wrappers) producing `Budget`-aware output.
- [ ] All three are stateless: running the same command twice against unchanged files produces the same output (no `Session`/history influence).
- [ ] Tests write fixture files to a temp directory and invoke `cli::run` with real paths, asserting rendered output — no `CommandExecutor` involved for these three verbs.
- [ ] A fixture covers a file too large to fit the budget, asserting the head/tail-preserved, omission-marked result.
