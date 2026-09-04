# Omission markers are Chunks, not a separate render-time concept

The Hard Budget Invariant requires omission markers to count against the token budget "from the same pool as everything else." We considered giving markers their own bookkeeping path, computed only at render time, separate from the `Chunk`/`ChunkKind` selection model. Instead, a marker is a real `Chunk` (`ChunkKind::OmissionMarker`) that goes through the same scoring, selection, and budget-accounting code as every other chunk.

## Considered Options

- Separate render-time artifact with its own token reservation (`marker_reserve`) computed outside the chunk model.
- A `Chunk` like any other, carrying the marker text as its payload.

We picked the latter: it keeps one selection/accounting path instead of two, and makes the "same pool as everything else" invariant literally true in the data model rather than something the renderer has to separately guarantee.
