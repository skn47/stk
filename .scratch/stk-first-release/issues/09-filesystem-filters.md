# 09: Filesystem filters — `stk read` / `stk grep` / `stk find`

**What to build:** `stk read <file>`, `stk grep ...`, and `stk find ...` as native, `Budget`-aware filters operating directly on the real filesystem (not wrapping `cat`/`rg`/`find` the way `rtk`'s versions do). Since these don't spawn a child process, they're tested against real temp-directory fixture files, not the `CommandExecutor` seam.

**Blocked by:** 06

**Status:** done

- [x] `stk read <file> --budget N` renders a token-bounded view of the file using conservative text-based slicing: a counter-verified binary search for the largest symmetric head+tail line count that fits, with an omission marker for the dropped middle. No Tree-sitter.
- [x] `stk grep <pattern> <path>` and `stk find <path> [--name PATTERN]` are native implementations (substring matching only, no regex engine dependency) producing `Budget`-aware output via a shared sequential (keep-from-start, one marker for the rest) truncation, distinct from `read`'s head/tail shape.
- [x] All three are stateless: running the same command twice against unchanged files produces the same output (no `Session`/history influence) — no state exists to influence them.
- [x] Tests write fixture files to a temp directory and invoke `cli::run` directly, asserting `FakeExecutor.invocations()` stays empty and rendered output is correct — no `CommandExecutor` involved for these three verbs, per the ticket's explicit direction (a deliberate, documented exception to the "integration tests spawn the real binary" convention, since these verbs never touch it).
- [x] A fixture covers a file too large to fit the budget, asserting the head/tail-preserved, omission-marked result.

**Implementation notes:**
- `grep`/`find` share a new `verbs::file_walk::walk_files` recursive walker (skips `.git`, doesn't follow symlinks) rather than each reimplementing directory traversal.
- Code review caught and fixed 4 issues: `truncate_sequential` (used by `grep`/`find`) always reserved marker-token space before checking whether the full content already fit, unlike `truncate_head_tail`'s early-return shortcut — spurious truncation on content that didn't need it, reproduced and fixed with the same early-fit check; `grep`'s directory walk treated *any* file-read error (including the root path simply not existing) as "not valid UTF-8, skip," so `stk grep pattern /nonexistent/path` silently exited 0 with empty output instead of erroring, reproduced and fixed; `find`'s walk only checked `is_dir()`, so a nonexistent path (or a symlink loop hitting the OS path-length limit) fell into the "must be a file" branch and was listed as a bogus result, reproduced and fixed; and `grep`/`find` had independently duplicated the same tree-walk logic (including the `.git` skip) — extracted into the shared `walk_files` helper, which also fixed the previous two bugs at the root (an explicit `symlink_metadata` check up front) rather than needing per-verb patches.
