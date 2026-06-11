# Implemented Examples

Status: implemented

This file records complete examples that are expected to type-check and run
against the implemented language specification.

## Line-Item Order Summary

The comparison example is implemented in `examples/comparison/`. Its rationale
is recorded in
[Comparison Example Task](../reference/source-decisions/records/result-comparison-example-task.md).

The pure API is:

```veln
summarize_order(lines: Vec<String>, catalog: Dict<String, Int>) -> Result<{item_count: Int, subtotal_cents: Int}, {kind: String, input: String}>
```

Input lines use `sku,quantity` spelling. The implementation rejects malformed
rows, non-integer or non-positive quantities, and unknown SKUs. The command
wrapper keeps stdout in `main` and leaves parsing and summarization in pure
functions.

The example uses these implemented language features together:

- dictionary lookup with `dict_get`
- fallible vector traversal with `vec_try_map_with`
- summary accumulation with `vec_fold`
- `Result` propagation
- record-shaped success and error values
- `stdio::println` for the wrapper
- a separate partial-program variant with a constrained typed hole
- canonical `#` source comments on example-authored notes

## Binary Fixture Records

The executable specification case
`../../examples/specification/run/binary-fixture-records/` keeps named valid
and invalid binary fixtures inside the example tree. The fixture records carry
the fixture name, decoded `ByteChunk`, optional consumed `ByteCount`, and
expected invalid-fixture error text without adding production standard-library
API beyond `byte_chunk_from_hex`.

The toolchain harness checks each named fixture through complete lowercase hex
in `case.toml`, plus decoded byte count and optional consumed count. Valid
fixture records keep the source-owned `ByteChunk` separate from the lowercase
hex expectation used for machine comparison. Invalid fixture records are
checked by their stable error text. This is executable specification evidence
for fixture ownership and expected-output comparison, not a public
serialization surface.

`../../examples/specification/run/binary-fixture-truncated-input-json/` shows a
named fixture record whose valid decoded bytes are intentionally too short for
the read under test. The case metadata keeps the fixture name, complete
lowercase hex, decoded byte count, expected consumed count, byte offset,
expected byte count, available byte count, readiness, and empty direct-read
field path separate from the `codec.incomplete_input` JSON assertion.

## Binary Byte Views

The executable specification case
`../../examples/specification/run/binary-byteview/` covers source-visible
`ByteView` slices, checked unsigned big-endian reads, checked unsigned
big-endian writes, truncation failures, range failures, and conversion
overflow failures without relying on HTTP/2 or codec declarations. It also
passes a `ByteView` through a channel and reads the received view to cover the
ordinary immutable freeze boundary.

The sibling failure cases under `../../examples/specification/run/` pin the
runtime `Result` propagation shape for ByteView read truncation, ByteView range
failure, and unsigned write conversion overflow in JSON and human command
output. The read-truncation JSON case also pins the `codec.incomplete_input`
byte diagnostic details. The named-fixture truncation case pins the same
diagnostic shape while proving that valid fixture bytes fail as codec
truncation, not as fixture text validation.
