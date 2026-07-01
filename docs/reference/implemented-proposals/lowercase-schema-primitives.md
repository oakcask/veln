# Lowercase Schema Primitives

Status: implemented

This record preserves the completed lowercase schema primitive migration. The
current source and execution behavior is specified by
`../../specification/source-surface.md`, `../../specification/execution.md`,
and `../../specification/commands.md`; this file is history and completion
evidence, not the source of current behavior.

## Completed Behavior

Binary schema field type text has canonical lowercase spellings for exact
width unsigned fields, visible flag bitsets, reserved-bit fields, repeated
fields, and supported dispatch payload field text:

```text
uint<width><endian?>
flag<width><endian?>
uint<width><endian?> reserves <value>
[<payload field type>; <count expression>]
```

The compatibility spellings remain accepted in schema-only positions:
`UIntN`, `UIntNbe`, `UIntNle`, `FlagN`, `FlagNbe`, `FlagNle`,
`ReservedBits(width, value)`, and `Repeat(count, Payload)`. They normalize to
the same descriptors and generated decode, encode, repeat, dispatch, and
derived codec behavior as the canonical lowercase spelling.

`veln fmt` writes supported compatibility field text in `format binary`
schemas as canonical lowercase spelling. The formatter covers direct fields,
representable reserved fields, repeated fields, and dispatch payload field
text while preserving ordinary format-neutral schema type names.

## Evidence

- Parser and formatter coverage:
  `crates/veln-syntax/src/tests.rs`.
- CLI formatter coverage:
  `crates/veln-cli/tests/check_json/fmt.rs`.
- Formatter executable specification:
  `examples/specification/fmt/canonical-formatting/`.
- Lowercase primitive diagnostics:
  `examples/specification/check/lowercase-schema-primitive-diagnostics/`.
- Lowercase reserved diagnostics:
  `examples/specification/check/lowercase-schema-reserves-diagnostics/`.
- Canonical repeat diagnostics and execution:
  `examples/specification/check/schema-repeat-canonical-syntax-diagnostics/`
  and
  `examples/specification/run/binary-schema-canonical-repeat-decode-encode/`.
- Legacy `Repeat(count, Payload)` lowercase payload execution:
  `examples/specification/run/binary-schema-repeat-decode/` and
  `examples/specification/run/binary-schema-repeat-encode/`.
- Dispatch payload execution:
  `examples/specification/run/binary-schema-closed-dispatch-decode/`,
  `examples/specification/run/binary-schema-extension-dispatch-decode/`,
  `examples/specification/run/binary-schema-extension-dispatch-encode/`, and
  `examples/specification/run/binary-schema-lowercase-reserved-dispatch-payload-decode-encode/`.

## Non-Goals

This migration did not add signed integers, floating-point fields,
variable-length integers, text encodings, arbitrary bitstream parsing, or new
byte widths beyond the implemented helper surface. It also did not remove
compatibility parsing for existing upper-case primitive names,
`ReservedBits(width, value)`, or `Repeat(count, Payload)`.
