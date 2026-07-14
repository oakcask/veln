# Integer Bitwise Operators

Status: implemented

This record describes the completed addition of bitwise operators on `Int`.
Current behavior is specified under `../../specification/`; this file is
retained for completion history.

## Goals

- Add a complete, backend-independent bitwise operator surface for `Int`.
- Give bitwise operations fixed semantics rather than inheriting backend shift
  or overflow behavior.
- Keep byte width and byte order in binary schemas while decoded exact-width
  unsigned values use `Int`.

## Operator Surface

The following operators are supported:

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
- `<<` shifts left and discards bits beyond the fixed integer width.
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
as `Int`; this work does not expand the source-visible integer range.

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

## Tooling

Formatter, editor token classification, documentation generation, repair
candidates, and command output recognize the operators.

## Non-Goals

- Adding unsigned source integer types or preserving schema width in ordinary
  expression types.
- Adding user-declared named bitfield declarations.
- Overloading bitwise operators for `Bool`, collections, byte containers, or
  user-defined ADTs.
- Changing exact-width unsigned schema representability or extending `Int`
  beyond its existing source-visible range.
- Treating `and`, `or`, or `not` as integer bitwise operators.

## Completion Evidence

- Lexer, parser, AST, formatter, type analysis, lowering, IR, and every backend
  implement the operators and precedence.
- Static and runtime invalid-shift behavior has human and structured diagnostic
  coverage.
- Constant folding, contract analysis, repair reasoning, documentation tools,
  editor support, and code metrics understand the operators.
- Executable examples demonstrate inspection, setting, clearing, and toggling
  bits on decoded exact-width unsigned fields in both byte orders.
