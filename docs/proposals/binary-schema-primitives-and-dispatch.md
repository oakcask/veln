# Binary Schema Primitives And Dispatch

Status: proposed

This proposal tracks remaining binary schema primitive and dispatch behavior.
Current implemented behavior is specified under
`../specification/source-surface.md`, `../specification/execution.md`, and
checked examples under `../../examples/specification/`.

The planned replacement of visible flag primitives with unsigned fields and
`Int` bitwise operators belongs to
[Integer Bitwise Operators And Flag Removal](integer-bitwise-operators-and-flag-removal.md),
not to the remaining primitive work tracked here.

Completed primitive, flag, reserved-bit, repeat, byte-view, dispatch, and
recursive dispatch helper slices are archived under
`../reference/implemented-proposals/`.
The completed binary schema anonymous record encode slice is archived under
[Binary Schema Anonymous Record Encode](../reference/implemented-proposals/binary-schema-anonymous-record-encode.md).
The completed bounded direct dispatch payload slice for zero-reserved subbyte
payloads from `uint1 reserves 0` through `uint7 reserves 0` is archived under
[Binary Schema Dispatch Lowercase Subbyte Reserved Payloads](../reference/implemented-proposals/binary-schema-dispatch-lowercase-subbyte-reserved-payloads.md).
The completed bounded direct dispatch payload slice for nonzero subbyte
payloads from `uint1 reserves 1` through `uint7 reserves 127`, with each value
bounded by its declared width, is archived under
[Binary Schema Dispatch Nonzero Lowercase Subbyte Reserved Payloads](../reference/implemented-proposals/binary-schema-dispatch-nonzero-lowercase-subbyte-reserved-payloads.md).
The completed `UInt40be` and `UInt40le` exact-width primitive slice is archived
under
[Binary Schema UInt40 Primitives](../reference/implemented-proposals/binary-schema-u40-primitives.md).
The completed `UInt48be` and `UInt48le` exact-width primitive slice is archived
under
[Binary Schema UInt48 Primitives](../reference/implemented-proposals/binary-schema-u48-primitives.md).
The completed bounded `Repeat` representation-only lowercase reserved payload
slice is archived under
[Binary Schema Repeat Helper Bindings](../reference/implemented-proposals/binary-schema-repeat-schema-payload-helpers.md).
The completed same-module recursive repeated nested payload helper coverage is
archived under the same repeat helper record.
The completed direct visible `UInt16be`, `UInt24be`, `UInt31be`, `UInt32be`,
`UInt56be`, and `UInt64be` generated helper parity slices are archived under
[Binary Schema Big-Endian Width Parity](../reference/implemented-proposals/binary-schema-big-endian-width-parity.md).
The completed direct visible `UInt56le` and `UInt64le` generated helper parity
slice is archived under
[Binary Schema UInt56le And UInt64le Parity](../reference/implemented-proposals/binary-schema-u56le-u64le-parity.md).
The completed visible-only packed eight-byte group slice is archived under
[Binary Schema Packed Visible Eight-Byte Groups](../reference/implemented-proposals/binary-schema-packed-visible-eight-byte-groups.md).
The completed bounded direct reserved-byte-prefix rule is archived under
[Binary Schema General Reserved Byte Prefixes](../reference/implemented-proposals/binary-schema-general-reserved-byte-prefixes.md).
The completed schema-local field reference diagnostics slice is archived under
[Binary Schema Field Reference Diagnostics](../reference/implemented-proposals/binary-schema-field-reference-diagnostics.md).
Schema-level value projection is no longer part of this proposal: `map to` in
schema bodies is removed as recorded in
[Remove Schema Map To](../reference/implemented-proposals/remove-schema-map-to.md).

## Problem

Binary protocols need compact source syntax for external representation
layouts without turning schema declarations into general protocol code.
Schemas should describe the byte layout, local validation, and helper
eligibility rules. Domain projection should stay in ordinary Veln functions or
explicit schema operations.

## Scope

Remaining work belongs here when it adds or completes binary schema behavior
for:

- exact-width unsigned primitive families
- representation-only fixed or reserved bits
- length-bounded `ByteView` fields and payload multiple validation
- bounded `Repeat` fields
- nested schema payload helpers
- closed and extension dispatch payload helpers
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
fields or supported literal constraints. Completed declaration-time diagnostics
for invalid, forward, missing, or wrong-role schema-local field references are
archived in the implemented proposal record.

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
- New visible flag wrappers or checked flag-helper families; the existing
  family is removed by the dedicated bitwise-operator proposal.
- Signed integers, floating-point encodings, variable-length integers, or text
  encodings before a concrete protocol slice requires them.
- Protocol-state validation that belongs in ordinary source functions.

## Remaining Completion Criteria

- Current specification pages describe only schema-local visible record helper
  shapes for binary schema decode and encode.
- Executable examples cover any newly added primitive or dispatch behavior.
- Runtime diagnostics and any remaining declaration diagnostics report the
  failed binary-layout fact at the relevant source span or byte offset.
- Completed slices are promoted to implemented proposal records and removed
  from this active proposal route.
