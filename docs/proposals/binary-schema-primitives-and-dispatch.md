# Binary Schema Primitives And Dispatch

Status: proposed

This proposal tracks remaining binary schema primitive and dispatch behavior.
Current implemented behavior is specified under
`../specification/source-surface.md`, `../specification/execution.md`, and
checked examples under `../../examples/specification/`.

Completed primitive, flag, reserved-bit, repeat, byte-view, and dispatch
helper slices are archived under `../reference/implemented-proposals/`.
Schema-level value projection is no longer part of this proposal: `map to` in
schema bodies is removed as recorded in
[Remove Schema Map To](../reference/implemented-proposals/remove-schema-map-to.md).

## Problem

Binary protocols need compact source syntax for external representation
layouts without turning schema declarations into general protocol code.
Schemas should describe the byte layout, local validation, and helper
eligibility rules. Domain projection should stay in ordinary Veln functions or
explicit codec implementations.

## Scope

Remaining work belongs here when it adds or completes binary schema behavior
for:

- exact-width unsigned primitive families
- visible flag bitset primitives and checked bit helpers
- representation-only fixed or reserved bits
- length-bounded `ByteView` fields and payload multiple validation
- bounded `Repeat` fields
- nested schema payload helpers
- closed and extension dispatch payload helpers
- declaration-time diagnostics for schema-local field references used by
  binary primitives
- runtime diagnostics for byte offsets, field paths, truncation,
  representability, dispatch mismatch, and schema validation failures

Implemented helpers operate on schema-local visible records. Representation
facts that are not visible fields remain available for validation and
diagnostics according to the primitive vocabulary, but they are not projected
through schema-level mapping clauses.

## Discussion Result: Primitive Boundaries

Exact-width unsigned primitives decode to ordinary `Int` values unless a
primitive defines a visible wrapper type. Encode helpers accept the same
schema-local visible shape and report structured representability failures
when a value cannot fit the declared width.

Reserved-bit and fixed-field primitives are representation facts. They decode
and encode the declared bit pattern, report focused mismatch diagnostics, and
do not create visible value fields unless the primitive explicitly defines a
visible source shape.

`ByteView` and `Repeat` forms may depend only on eligible earlier visible
fields or supported literal constraints. Invalid, forward, missing, or
wrong-role field references should be rejected at declaration time when the
relationship is statically visible.

## Discussion Result: Dispatch Boundaries

Closed dispatch chooses a payload case from an earlier visible tag field and
rejects unknown tags. Extension dispatch may preserve unknown payloads when
the selected representation vocabulary supports that behavior.

Nested payload helpers are eligible only when the nested schema can expose the
needed decode or encode helper over its schema-local visible shape. Recursive
payload support requires a length-bounded parent field and a non-recursive
base case so helper derivation remains finite.

## Non-Goals

- Schema-level `map to` syntax, mapping expressions, selected mappings,
  mapping projection diagnostics, or inverse projection rules.
- General bitstream parsing outside the declared primitive vocabulary.
- Signed integers, floating-point encodings, variable-length integers, or text
  encodings before a concrete protocol slice requires them.
- Protocol-state validation that belongs in ordinary source functions.

## Remaining Completion Criteria

- Current specification pages describe only schema-local visible record helper
  shapes for binary schema decode and encode.
- Executable examples cover any newly added primitive or dispatch behavior.
- Runtime and declaration diagnostics report the failed binary-layout fact at
  the relevant source span or byte offset.
- Completed slices are promoted to implemented proposal records and removed
  from this active proposal route.
