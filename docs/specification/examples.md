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
in `case.toml`, plus decoded byte count and optional consumed count. Invalid
fixture records are checked by their stable error text. This is executable
specification evidence for fixture ownership and expected-output comparison,
not a public serialization surface.
