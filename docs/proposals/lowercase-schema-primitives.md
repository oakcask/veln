# Lowercase Schema Primitives

Status: partially implemented

The direct `format binary` field spelling slice for `uint...` and `flag...`
is implemented under `../specification/source-surface.md`,
`../specification/execution.md`, and checked by
`../../examples/specification/check/lowercase-schema-primitive-diagnostics/`
plus
`../../examples/specification/run/binary-schema-lowercase-primitives-decode/`.

This proposal tracks the remaining lower-case schema primitive work: reserved
field suffixes, repeated-field array syntax, nested payload positions, and
formatter migration. Existing upper-case spellings such as `UInt24be` and
`Flag16le` remain schema-only compatibility forms.

The goal is to keep binary representation vocabulary out of the ordinary
source type namespace and to avoid implementation tables that enumerate one
token or primitive name per supported width.

## Problem

Current binary schema field declarations use exact-width spellings such as:

```veln
schema Http2FrameHeaderWire
	format binary
	length: UInt24be
	kind: UInt8
	flags: Flag8
	stream_id: UInt31be
end
```

Those names are schema-local representation vocabulary, not ordinary Veln
types. However, the leading upper-case spelling makes them look like source
types or constructors, and implementations are encouraged to recognize each
supported width through an `exact_width_schema_primitive`-style table.

That table becomes awkward whenever the schema language adds another width,
another endian variant, or another primitive family. The parser should be able
to recognize the structure of the primitive spelling and pass a normalized
primitive descriptor to semantic checking instead.

The same schema-local representation vocabulary currently includes
`Repeat(count, Payload)`, which looks like an ordinary constructor or function
call even though it is only valid in schema field type positions. The repeated
payload should use field-type syntax instead of borrowing a source-like call
surface.

## Remaining Proposed Syntax

Direct `format binary` schema fields already accept the canonical unsigned and
flag primitive spellings:

```text
uint<width><endian?>
flag<width><endian?>
```

The remaining reserved-field spelling is:

```text
uint<width><endian?> reserves <value>
```

Examples:

```veln
schema Http2FrameHeaderWire
	format binary
	length: uint24be
	kind: uint8
	flags: flag8
	stream_reserved: uint1 reserves 0
	stream_id: uint31be
end
```

The remaining nested payload work keeps the spellings valid only where the
schema grammar expects a nested schema payload type. They are not ordinary
source identifiers, type names, constructors, functions, imports, or public
aliases.

The direct-field implementation decomposes the lower-case token into:

- primitive family: `uint` or `flag`
- decimal width
- optional byte order suffix: `be` or `le`

Remaining suffix and nested-payload work should reuse the same normalized
descriptor shape instead of enumerating each width spelling:

```text
SchemaPrimitive {
  family: uint,
  width_bits: 24,
  byte_order: be,
}
```

The optional `reserves <value>` suffix changes a parsed `uint` field type into
a reserved-bit primitive. For example:

```veln
schema ReservedHeader
	format binary
	unused: uint32be reserves 0
end
```

is the canonical spelling for a byte-ordered thirty-two-bit reserved field
whose expected value is zero. The suffix is not a fixed-value constraint on a
visible unsigned field. It is schema-local representation vocabulary
equivalent to the compatibility `ReservedBits(width, value)` primitive after
normalization.

The semantic checker decides whether the parsed width and any remaining
reserved suffix are accepted by the implemented binary schema helper surface.

Repeated schema fields use the canonical form:

```text
[<payload field type>; <count expression>]
```

Examples:

```veln
schema CountedValues
	format binary
	count: uint8
	items: [uint16be; count]
end

schema NestedValues
	format binary
	count: uint8
	items: [ItemRecord; count]
end

schema SizedViews
	format binary
	count: uint8
	item_length: uint16be
	items: [ByteView(item_length); count]
end
```

The payload appears before the semicolon and the count expression appears
after it. This intentionally reverses the compatibility `Repeat(count,
Payload)` argument order so the canonical form reads as "a list of payload
values with this count", matching Rust array type spelling. The count
expression keeps the existing schema repeat expression surface: an earlier
count field, or one of the already implemented arithmetic forms over earlier
count fields. The payload field type may be a lower-case primitive, a
compatibility primitive, a nested schema type, a `ByteView(...)` payload, or a
later schema-only payload form accepted by semantic checking.

## Remaining Field Type Positions

Lower-case schema primitives still need to be accepted in these schema-only
positions where current exact-width primitives are accepted:

- `[Payload; count]` repeated payload fields
- `match tag ... end` and `match tag bounded by length ... end` payload cases
- `match extension tag bounded by length ... end` payload cases
- any later schema-only composition form that explicitly accepts binary
  primitive field types

They remain gated by `format binary`. Format-neutral schemas must continue to
reject binary-only primitive vocabulary.

## Width And Endian Rules

The direct-field implementation and the remaining reserved and nested-payload
work share these semantic rules:

- `uint1` through `uint7` are sub-byte unsigned integer fields.
- `uint8` is the one-byte unsigned integer field.
- `uint16be`, `uint16le`, `uint24be`, `uint24le`, and other implemented
  byte-width unsigned fields declare fixed-width byte-ordered fields.
- `uint31be` and `uint31le` keep their current meaning as 31-bit unsigned
  fields in a byte-ordered storage position whose remaining bit is supplied by
  an adjacent reserved or fixed field layout.
- `flag8` is the one-byte visible flag bitset field.
- `flag16be`, `flag16le`, `flag24be`, `flag24le`, and other implemented
  byte-width flag fields declare fixed-width byte-ordered bitsets.
- Multi-byte `uint` and `flag` fields require `be` or `le`.
- `uint8be`, `uint8le`, `flag8be`, and `flag8le` are rejected as redundant
  byte-order suffixes unless a later proposal gives them a distinct meaning.
- Unsupported widths are parsed and then rejected by semantic checking with a
  width-specific diagnostic.
- `reserves <value>` is accepted only on `uint` primitives. The value must be
  a literal non-negative integer.
- `uint` fields with `reserves <value>` keep the same endian requirements as
  visible `uint` fields: sub-byte widths and `uint8` omit endian, while
  multi-byte widths such as `uint16be`, `uint32be`, and `uint64le` require
  endian.
- A `uint` field with `reserves <value>` is representation-only. Decode and
  encode helpers validate or emit the declared value, omit the field from
  ordinary decoded and encoded records, and expose the validated value as an
  `Int` mapping source only when a mapping assignment explicitly names the
  reserved field.
- The field path for reserved-bit diagnostics uses the declared field name.

The accepted width set matches the implemented upper-case and
`ReservedBits(width, value)` surface. This proposal changes spelling and
parsing structure; it does not by itself expand the executable primitive set.

## Compatibility Spelling

Existing upper-case spellings remain schema-only compatibility spellings:

```veln
length: UInt24be
flags: Flag16le
unused: ReservedBits(32, 0)
```

Implementations should normalize both compatibility and canonical spellings to
the same schema primitive descriptor. `ReservedBits(width, value)` normalizes
to the same reserved primitive descriptor as `uint<width><endian?> reserves
<value>` when the canonical spelling can represent that layout. The upper-case
forms and `ReservedBits` should not become ordinary source-visible types,
constructors, or functions.

Existing `Repeat(count, Payload)` spelling remains a schema-only compatibility
spelling for repeated payload fields:

```veln
items: Repeat(count, UInt16be)
```

Implementations should normalize it to the same repeat descriptor as:

```veln
items: [uint16be; count]
```

The compatibility form preserves its historical argument order. The canonical
form always writes the payload before the semicolon and the count expression
after it.

The formatter should prefer the lower-case canonical spelling for newly
formatted schema field types after the migration point is chosen. Before that
point, a formatter may preserve written upper-case, `ReservedBits`, and
`Repeat(count, Payload)` compatibility spelling to avoid unnecessary churn in
existing examples.

## Diagnostics

Diagnostics should report the specific failed fact at the field type span.

Examples:

- `uint24` fails because a multi-byte unsigned field is missing byte order.
- `flag32` fails because a multi-byte flag field is missing byte order.
- `uint8be` fails because a one-byte unsigned field must not specify byte
  order.
- `uint9` fails because the width is not supported by the implemented helper
  surface.
- `flag32be reserves 0` fails because reserved fields must use a `uint`
  primitive.
- `unused: uint32 reserves 0` fails because a multi-byte reserved unsigned
  field is missing byte order.
- `unused: uint8 reserves -1` fails because a reserved value must be a
  non-negative integer literal.
- `items: [uint16be count]` fails because a repeated field type is missing the
  semicolon between the payload and count expression.
- `items: [uint16be; missing_count]` fails when the count expression does not
  name an earlier visible count field accepted by the implemented helper
  surface.
- `packet: uint24be` outside a `format binary` schema fails because binary
  primitive vocabulary is not available in that schema format.

Related notes may explain that the spelling is schema-only and should not be
imported, aliased, or used as an ordinary Veln type.

## Remaining Migration Plan

The direct field parser, normalization, focused direct-field diagnostics, and
direct-field executable examples are implemented. The remaining work can be
staged without changing binary schema semantics:

1. Parse `reserves <value>` after `uint` field types and normalize it with
   compatibility `ReservedBits(width, value)` fields.
2. Parse `[Payload; count]` repeated field types and normalize them with
   compatibility `Repeat(count, Payload)` fields.
3. Extend lower-case primitive normalization to repeated and dispatch payload
   positions that already accept compatibility primitive spellings.
4. Keep helper generation, mapping, encode, decode, and diagnostic behavior
   unchanged after normalization.
5. Add executable examples for reserved fields, nested payload cases, repeated
   payloads, and rejection diagnostics.
6. Teach the formatter to write lower-case canonical spelling and
   `[Payload; count]` repeated field syntax when the project is ready to
   migrate checked examples.
7. Update `../specification/source-surface.md`,
   `../specification/execution.md`, and matching examples for each remaining
   implemented slice.

## Non-Goals

This proposal does not add signed integers, floating-point fields,
variable-length integers, text encodings, arbitrary bitstream parsing, or new
byte widths beyond the already implemented helper surface.

This proposal does not change the source-visible wrapper types used by
implemented flag helper functions. For example, a decoded `flag16be` field may
still map to the existing source-visible flag wrapper value until a separate
proposal changes that runtime value surface.

This proposal does not make `reserves` a general-purpose field constraint for
ordinary visible values. It is reserved for schema-local reserved-bit
representation fields.

## Remaining Completion Criteria

The proposal is complete when:

- Direct-field behavior remains covered by `../specification/` and executable
  examples.
- `uint<width><endian?> reserves <value>` normalizes with compatible
  `ReservedBits(width, value)` fields.
- `[Payload; count]` repeated field syntax is accepted only in schema field
  type positions.
- Lower-case primitives normalize in repeated payload and dispatch payload
  positions that already accept compatibility primitive spellings.
- Existing `ReservedBits(width, value)` spellings continue to work as
  schema-only compatibility spellings and normalize to the same descriptor as
  canonical reserved fields.
- Existing `Repeat(count, Payload)` spellings continue to work as schema-only
  compatibility spellings and normalize to the same descriptor as canonical
  repeated fields.
- Generated decode, encode, mapping, repeat, dispatch, and derived codec
  helper behavior is unchanged after each remaining normalization slice.
- Formatter, editor token, source-surface documentation, execution
  documentation, and executable specification examples reflect the canonical
  lower-case spelling and repeated field syntax.
