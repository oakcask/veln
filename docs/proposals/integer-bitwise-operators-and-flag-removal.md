# Integer Bitwise Operators And Flag Removal

Status: proposed

This proposal replaces the source-visible `FlagN` family with bitwise
operators on `Int`. Current implemented behavior remains specified under
`../specification/` until this proposal is implemented.

## Problem

Binary schema `flag...` fields currently decode to nominal `FlagN` wrapper
types. Each wrapper has one constructor carrying `bits: Int`, and its helper
family only exposes raw-bit conversion, bit inspection, and setting one bit.
Constructor patterns therefore unwrap an integer rather than describe named
flags, and public constructors do not preserve the width invariant.

Exact-width `uint...` fields already decode to `Int`. If `Int` has ordinary
bitwise operators, the flag wrappers add little expressive power while
duplicating every supported width and byte order across types, constructors,
prelude functions, runtime functions, generated schema helpers, backend
mappings, diagnostics, and examples.

Veln should provide general integer bit operations and keep byte width and byte
order in the binary schema where those representation facts belong.

## Goals

- Add a complete, backend-independent bitwise operator surface for `Int`.
- Replace binary schema `flag...` fields with the corresponding `uint...`
  fields.
- Decode every visible exact-width unsigned schema field to `Int`.
- Remove the `FlagN` types, constructors, patterns, helpers, runtime support,
  and compatibility schema spellings.
- Give bitwise operations fixed semantics rather than inheriting backend shift
  or overflow behavior.
- Provide focused migration diagnostics for removed flag vocabulary.

## Operator Surface

The following operators are added:

```text
~value
left & right
left | right
left ^ right
left << count
left >> count
left >>> count
```

All operands have type `Int`, and every operator returns `Int` except that
expressions comparing a masked result continue to use the existing comparison
and equality rules.

- `~` complements every bit.
- `&`, `|`, and `^` compute bitwise AND, OR, and XOR.
- `<<` shifts left and discards bits shifted beyond the fixed integer width.
- `>>` performs an arithmetic right shift and extends the sign bit.
- `>>>` performs a logical right shift and fills high bits with zero.

The lexer uses longest-token matching, so `|>` remains the pipeline operator,
while a standalone `|` is bitwise OR. Likewise, `>>>`, `>>`, `>=`, and `>` are
distinct tokens selected longest first.

## Integer And Shift Semantics

Bitwise operators view `Int` as a signed 64-bit two's-complement value. Results
are the signed `Int` interpretation of the resulting 64-bit pattern. Bitwise
left shift deliberately discards high bits and does not report arithmetic
overflow.

A shift count must be in the inclusive range `0` through `63`. A literal count
outside this range is a type diagnostic at the count expression. A value that
is outside this range at runtime fails with `runtime.invalid_shift_count`; it
is not masked modulo 64 and does not inherit host behavior. Human output names
the invalid count and the permitted range, while structured output includes
the operator, actual count, minimum count, and maximum count.

These rules make all operators deterministic across backends. Binary schema
values remain nonnegative when their external unsigned value is representable
as `Int`; this proposal does not expand the source-visible integer range.

## Precedence And Formatting

Operator precedence, from highest to lowest, is:

1. prefix `not`, unary `-`, and `~`
2. `*` and `/`
3. `+` and `-`
4. `<<`, `>>`, and `>>>`
5. `<`, `<=`, `>`, and `>=`
6. `==` and `!=`
7. `&`
8. `^`
9. `|`
10. `and`
11. `or`
12. `|>`

The formatter emits one space around binary bitwise and shift operators and no
space between unary `~` and its operand. Parentheses are preserved or inserted
when required by this precedence table.

## Binary Schema Changes

The lowercase `flag8`, `flag16be`, `flag16le`, `flag24be`, `flag24le`,
`flag32be`, `flag32le`, `flag40be`, `flag40le`, `flag48be`, `flag48le`,
`flag56be`, `flag56le`, `flag64be`, and `flag64le` schema primitives are
removed. Their uppercase compatibility spellings are removed with them.

Each field migrates to its existing unsigned primitive:

```text
flag8     -> uint8
flag16be  -> uint16be
flag16le  -> uint16le
flag24be  -> uint24be
flag24le  -> uint24le
flag32be  -> uint32be
flag32le  -> uint32le
flag40be  -> uint40be
flag40le  -> uint40le
flag48be  -> uint48be
flag48le  -> uint48le
flag56be  -> uint56be
flag56le  -> uint56le
flag64be  -> uint64be
flag64le  -> uint64le
```

Direct fields, repeated payloads, anonymous records, nested schemas, closed
dispatch, extension dispatch, explicit schema operations, and derived codec
helpers all use the same replacement. Decoded flag fields consequently change
from `FlagN` to `Int`; encode inputs accept `Int` and retain the existing
schema-owned representability checks for the declared unsigned width.

Endianness continues to describe byte encoding only. It does not affect the
bit numbering of the decoded integer: bit zero is the least-significant bit of
the value for both big-endian and little-endian schema fields.

## Removed Source Vocabulary

The following are removed rather than deprecated or retained as aliases:

- built-in `Flag8`, `Flag16be`, `Flag16le`, `Flag24be`, `Flag24le`,
  `Flag32be`, `Flag32le`, `Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`,
  `Flag56be`, `Flag56le`, `Flag64be`, and `Flag64le` types
- constructors and constructor patterns with those names
- every `flagN_is_set`, `flagN_set`, `flagN_bits`, and `flagN_from_bits`
  prelude helper, including endian-qualified forms
- compiler-known descriptors, runtime entry points, backend mappings, and
  generated helper special cases dedicated to those types

No compatibility aliases remain because aliases would preserve the duplicate
type boundary and keep old code dependent on semantics this proposal removes.

## Source Migration

Ordinary uses migrate as follows:

```veln
# Before
let padded: Bool = flag8_is_set(packet.flags, 3)?
let updated: Flag8 = flag8_set(packet.flags, 5)?

# After
let padded: Bool = (packet.flags & (1 << 3)) != 0
let updated: Int = packet.flags | (1 << 5)
```

Raw-bit extraction and construction disappear:

```text
flagN_bits(value)       -> value
flagN_from_bits(value)? -> value
FlagN(value)            -> value
```

A constructor pattern that only extracts the payload becomes an ordinary
`Int` binding. Exhaustive matching over the single `FlagN` constructor has no
replacement because it expressed no choice.

The old checked helpers rejected bit indexes outside the selected flag width
and rejected raw values outside that width. After migration, the schema still
checks representability when encoding, while ordinary `Int` expressions do not
carry a schema width. Code that accepts a dynamic protocol-specific bit index
must validate that index against the protocol width explicitly. This loss of a
nominal width outside the schema is intentional.

## Diagnostics And Tooling

- Removed lowercase or uppercase flag primitives in a binary schema report a
  schema diagnostic that names the corresponding `uint...` replacement.
- Removed `FlagN` type, constructor, and pattern uses report a focused removed
  vocabulary diagnostic rather than a generic unresolved-name diagnostic.
- Removed helper calls name the operator expression that replaces the helper
  when the call shape permits a direct suggestion.
- Formatter, editor token classification, documentation generation, repair
  candidates, and command output recognize the new operators.
- Migration suggestions do not silently remove dynamic index or raw-value
  validation. When equivalence depends on a former width check, the diagnostic
  explains that an explicit range check is still required.

## Non-Goals

- Adding unsigned source integer types or preserving schema width in ordinary
  expression types.
- Adding user-declared named bitfield or bitflag declarations.
- Overloading bitwise operators for `Bool`, collections, byte containers, or
  user-defined ADTs.
- Changing exact-width unsigned schema representability or extending `Int`
  beyond its existing source-visible range.
- Treating `and`, `or`, or `not` as integer bitwise operators.
- Retaining deprecated `FlagN` aliases or helper shims.

## Completion Criteria

- Lexer, parser, AST, formatter, type analysis, lowering, IR, and every backend
  implement the proposed operators and precedence.
- Static and runtime invalid-shift behavior has human and structured diagnostic
  coverage.
- Constant folding, contract analysis, repair reasoning, documentation tools,
  editor support, and code metrics either understand the new operators or
  reject unsupported uses explicitly.
- Binary schema decode and encode use `uint...` and `Int` for every former flag
  field position, including nested, repeated, dispatch, and derived-codec
  paths.
- All `FlagN` descriptors, constructors, helpers, runtime support, backend
  mappings, compatibility spellings, and tests that assert supported flag
  behavior are removed or converted. Only the dedicated removal diagnostics
  and negative tests permitted below retain the removed vocabulary.
- Executable examples demonstrate inspection, setting, clearing, and toggling
  bits on decoded `uint...` fields in both byte orders.
- Current implementation code, standard-library source, backend code, editor
  definitions, workflow support, generated surfaces, README routes,
  `docs/specification/`, and ordinary `examples/specification/` cases contain
  no `FlagN` type, constructor, pattern, helper, or `flag...` schema primitive
  as accepted vocabulary or executable behavior. Dedicated removed-syntax
  diagnostic cases are the only exception under `examples/specification/`.
- Current specification and executable examples positively cover every
  replacement `uint...` field shape and the new bitwise operator behavior; the
  replacement is not complete if the new path exists only in implementation
  code or migration tests.
- A repository-wide residual-name audit searches exact names and patterned
  uppercase and lowercase flag-family prefixes across code, tests, examples,
  generated content, editor support, workflows, and documentation. Every hit
  is classified. The only permitted residual classifications are
  `historical-doc` under `docs/reference/implemented-proposals/` or the
  archived completion record, and `test-assertion` in migration diagnostics or
  dedicated removed-syntax tests. `compatibility-alias` and
  `follow-up-required` residuals prevent completion.
- Implemented behavior is promoted into `../specification/` and
  `../../examples/specification/`, this proposal is removed from the active
  proposal catalog, and its completed record is archived under
  `../reference/implemented-proposals/`.
