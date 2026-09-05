# 04: `stk pipe` — stdin + named filter, buffered-output path

**What to build:** `stk pipe [-f FILTER] [--passthrough]` reads stdin, applies one named filter, and prints the filtered result — the lightweight Unix-pipe mode, distinct from `compile`'s full scoring/budgeting. Since `pipe` never spawns a child process, this ticket is where the buffered-output half of the process-wrapper contract (as opposed to `run`/`proxy`'s live-streaming half) gets built, including graceful handling of a broken pipe on `stk`'s own stdout.

**Blocked by:** 01

**Status:** done

- [x] `stk pipe -f FILTER` reads all of stdin, applies the named filter, and prints the result to stdout. First-release filter catalog: `ansi` (strips ANSI CSI escape sequences) — STK-original, not an alias of any real `rtk` filter name (rtk's `-f` catalog is command-specific specialists this release doesn't have yet).
- [x] `--passthrough` prints the input unmodified — and, verified against the real `rtk pipe --help` and empirically, so does giving **no flags at all** (rtk's own default), which `stk pipe` now matches. Passthrough streams directly (`io::copy`) rather than buffering, unlike named filters which need the full input to operate on.
- [x] An unrecognized `-f FILTER` name fails with a clear error rather than silently passing through or crashing.
- [x] If `stk pipe`'s stdout is closed early downstream (e.g. piped into `head`), `stk` exits gracefully (exit 0) rather than crashing or hanging — verified with a deterministic test that closes the read end before the child ever writes.
- [x] Golden-comparison tests confirm `stk pipe` matches real `rtk pipe` for `--passthrough` and for the no-flags-given default (skipped gracefully without `rtk`).

**Implementation notes:**
- New `src/normalize/` module (matching the design doc's original §16 layout) holds `ansi::strip`, reusable by the fast-path work in ticket 05.
- Code review caught and fixed 3 issues: the ANSI stripper treated any byte after a malformed CSI's parameter bytes as a valid "final byte," so a truncated sequence immediately followed by a *real* escape sequence swallowed that real escape and leaked garbage into the output (verified with the actual binary, fixed by validating the final-byte range, regression-tested); `-f ansi` wasn't documented as STK-original despite pipe's "matches rtk pipe" framing, risking the false impression it aliases a real rtk filter (now a doc comment clarifies this); and passthrough was buffering the entire input and output in memory before writing anything, working against the design doc's own "avoid loading huge logs fully into memory" principle for exactly the large-log use case it calls out (now streams via `io::copy`).
