# Binary Schema Direct Nested Decode Bindings

Status: implemented

This record preserves the completed direct nested binary schema generated
helper binding slice from `../../proposals/schema-declaration-surface.md`.
Current behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, and the checked executable examples under
`../../../examples/specification/run/`.

## Outcome

Generated binary schema decode helpers accept a direct field whose type names
an eligible same-module or public imported binary schema declared through the
schema-aware path rules. Decode consumes the nested schema in place, stores the
nested schema-local visible record at the parent field, and advances the parent
byte position by the nested helper's consumed byte count.

Generated encode helpers accept the same nested visible record shape at the
parent field and write it through the nested schema helper. Runtime failures
from the nested helper preserve the parent schema field path before appending
the nested schema field path.

## Evidence

- `../../../examples/specification/run/binary-schema-direct-nested-decode/`
  checks direct nested decode and encode through the parent helper.
- `../../../examples/specification/run/binary-schema-direct-nested-truncated-json/`
  checks truncation diagnostics with the parent field path and nested schema
  field path.

## Remaining Work

The broader schema declaration surface proposal remains open for binary schema
fields outside the implemented exact-width unsigned primitive, visible flag
bitset, direct nested schema, bounded repeat, length-bounded `ByteView`,
closed dispatch, and extension dispatch slices, plus format-neutral fields
outside the implemented recursive visible-shape helper boundary.
