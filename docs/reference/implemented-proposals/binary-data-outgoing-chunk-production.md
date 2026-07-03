# Binary Data Outgoing Chunk Production

Status: implemented

This record preserves the completed budgeted outgoing chunk production slice
from the binary data standard-library proposal. Current behavior is specified
by `../../specification/names-effects.md`,
`../../specification/execution.md`, and the checked executable case
`../../../examples/specification/run/binary-output-chunk-production/`.

## Completed Behavior

`byte_chunks_produce(chunks, budget)` is a pure source-visible standard-library
helper for incremental output. It accepts a `List<ByteChunk>` and a
`ByteCount` budget, then returns a record containing:

- `chunks`: the ordered prefix of whole chunks that fit within the budget
- `produced`: the produced byte count
- `remaining`: the unchanged suffix for a later call

The helper never splits a `ByteChunk`. It returns no produced chunks for a
zero budget, leaves the whole input list in `remaining` when the first chunk
does not fit, and returns an empty remaining suffix when all chunks fit.

## Evidence

- `../../../examples/specification/run/binary-output-chunk-production/` checks
  full fit, partial fit, first-chunk-too-large, and zero-budget cases.
- The same checked case pins output chunk order and exact lowercase hex
  contents for both produced and remaining chunk lists.
- `../../specification/names-effects.md` and
  `../../specification/execution.md` summarize the current source-visible
  helper surface and execution behavior.
