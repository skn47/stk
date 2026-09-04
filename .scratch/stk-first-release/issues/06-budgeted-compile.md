# 06: `Budget`-bounded `stk compile` — `Chunk` model + Hard Budget Invariant + omission markers

**What to build:** `stk --budget N compile` adds the `Chunk` model and the full Budget Selection Algorithm on top of ticket 05's fast path: chunks are classified by priority (P0 fatal errors down to P5 noise), P0 is always included (structurally shrunk with a visible truncation marker if it alone exceeds budget), optional chunks are packed by score into whatever budget remains, and a final render-and-recount pass trims further if needed. Omission markers are themselves `Chunk`s (`ChunkKind::OmissionMarker`, ADR-0001), going through the same selection/accounting path as everything else. A `--budget` below the minimum viable floor is a clear refusal, not a degenerate render.

**Blocked by:** 05

**Status:** ready-for-agent

- [ ] `Chunk { id, kind, text, source, token_count, score, priority, relationships }` and `ChunkKind` (including `OmissionMarker`) exist and are populated from ticket 05's fast-path output.
- [ ] The `ApproximateCounter` implements `ceil(max(chars/3, words*1.4))` per ADR-0002, and is used for all budget accounting.
- [ ] Chunks are selected per the Budget Selection Algorithm: P0 mandatory chunks always included (shrunk with a visible marker if they alone exceed budget minus marker reserve); optional chunks packed by score into remaining space.
- [ ] A final render-and-recount pass re-measures actual rendered output and drops lowest-scoring optional chunks (never marker/P0 text) if the render pass pushed over budget.
- [ ] `--budget` values below `marker_reserve + min_p0_floor` are refused with a clear error rather than rendered.
- [ ] Removed content always leaves a visible omission marker (e.g. `[stk: omitted N repeated/success lines]`), and that marker is itself a selected `Chunk`, counted against the same budget.
- [ ] Fixture-driven tests cover: normal budgeted compression, P0-forced-to-shrink, and budget-below-floor-refused.
