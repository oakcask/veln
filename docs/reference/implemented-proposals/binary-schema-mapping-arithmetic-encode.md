# Binary Schema Mapping Arithmetic Encode

Status: implemented

This record preserves the completed arithmetic mapped-record encode slice from
`../../proposals/schema-declaration-surface.md` and
`../../proposals/binary-schema-primitives-and-dispatch.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

Generated `byte_encode_<schema>` helpers and matching `derive encode` codec
boundaries accept one structural mapping target record when a schema-local
visible `Int` field can be recovered from one mapped target `Int` field by a
narrow reversible arithmetic assignment.

The implemented reversible forms are:

- `target = field + literal`
- `target = literal + field`
- `target = field - literal`

The helper projects the supplied mapped target record back to the schema-local
visible field, then continues through the generated encode path. Primitive
range checks, field-local validation, byte order, and command-facing
diagnostics therefore remain owned by the existing schema encode boundary.

This slice does not add general symbolic solving, multiplication, division,
comparisons, boolean selection, converter-call arithmetic, or multi-field
arithmetic such as `left + right` to mapped encode projection.

## Evidence

- `../../../examples/specification/run/binary-schema-mapping-arithmetic-encode/`
  checks successful generated schema encode through `field + literal`,
  `literal + field`, and `field - literal` inverse projection.
- `../../../examples/specification/run/binary-schema-mapping-arithmetic-encode-out-of-range/`
  checks that a recovered schema-local value outside the declared primitive
  range reports the existing `codec.encode_value_unrepresentable` diagnostic at
  the schema-local field path.
- `../../../examples/specification/run/derived-codec-mapping-arithmetic-encode-boundary/`
  checks the same projection through a `derive encode` codec boundary.
- `crates/veln-sema/src/tests/prelude_and_callable_values.rs` checks generated
  helper eligibility and derived codec encode-step resolution for mapped
  arithmetic records.

## Remaining Work

The broader schema declaration and binary schema proposals remain open for
mapping, primitive, and dispatch behavior outside the implemented generated
helper slices.
