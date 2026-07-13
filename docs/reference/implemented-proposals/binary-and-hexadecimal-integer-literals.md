# Binary And Hexadecimal Integer Literals

Status: implemented

This record tracks the completed addition of binary and hexadecimal source
spellings for `Int` values. Current behavior is specified under
`../../specification/` and checked by examples under
`../../../examples/specification/`.

## Read First

- Current literal grammar, value behavior, and executable cases:
  [../../specification/source-surface.md](../../specification/source-surface.md).
- Current editor classification:
  [../../specification/editor-support.md](../../specification/editor-support.md).
- Current diagnostic routes:
  [../../specification/diagnostics-json.md](../../specification/diagnostics-json.md).

## Problem

Veln integer literals currently use decimal digits only. Code that describes
bit masks, binary schema constants, byte values, and protocol identifiers must
therefore translate naturally bit-oriented values into decimal. That obscures
the relationship between the source and the represented bits, especially when
the value is used with the proposed integer bitwise operators.

Binary and hexadecimal spellings should denote the same `Int` values as
decimal literals without introducing new integer types or backend-dependent
interpretations.

## Goals

- Accept `0b` binary and `0x` hexadecimal integer literals.
- Give all integer literal spellings the same `Int` type and value range.
- Use the new spellings consistently in expressions, patterns, and
  integer-literal schema arguments and constraints.
- Reject malformed prefixed literals as one focused source error rather than
  tokenizing a valid prefix of the text.
- Preserve an author's accepted radix spelling during formatting.

## Syntax

The integer literal grammar becomes:

```text
IntLiteral       ::= DecimalLiteral | BinaryLiteral | HexadecimalLiteral
DecimalLiteral   ::= DecimalDigit+
BinaryLiteral    ::= "0b" BinaryDigit+
HexadecimalLiteral ::= "0x" HexadecimalDigit+
DecimalDigit     ::= "0" ... "9"
BinaryDigit      ::= "0" | "1"
HexadecimalDigit ::= DecimalDigit | "a" ... "f" | "A" ... "F"
```

The prefixes are lowercase. `0B` and `0X` are not accepted aliases. Both
letter cases are accepted for hexadecimal digits, so `0xcafe`, `0xCAFE`, and
`0xCafe` have the same value. At least one digit must follow a prefix. Leading
zeroes are accepted in every radix.

Digit separators are not part of this proposal. An underscore in a numeric
candidate is invalid rather than ignored.

Examples of accepted literals include:

```veln
let mask: Int = 0b11010010
let frame_type: Int = 0x1a
let maximum_byte: Int = 0xFF
```

## Token Boundaries And Invalid Digits

After `0b`, `0B`, `0x`, or `0X`, the lexer treats contiguous ASCII letters,
digits, and underscores as one numeric candidate up to the normal token
boundary. An uppercase prefix reports that the prefix is unsupported. After a
lowercase prefix, any character that is not a digit for the selected radix
makes the entire candidate invalid.

Consequently, `0b102`, `0xg1`, `0x12z`, and `0b10_01` must not be split into a
shorter valid integer followed by another token. `0b` and `0x` are likewise
malformed integer candidates, not decimal zero followed by an identifier.

Prefixed floating-point literals are not added. A radix candidate immediately
followed by a decimal point and digit sequence, such as `0x1.2` or `0b1.0`, is
rejected as one malformed numeric sequence rather than interpreted as a
hexadecimal or binary floating-point value.

The existing decimal and floating-point token boundary remains unchanged
except where a token starts with one of the four prefix candidates above.

## Value And Type Semantics

Every accepted spelling has type `Int`. Its mathematical value is obtained by
interpreting the digits as an unsigned magnitude in radix 10, 2, or 16. The
result must fit the existing nonnegative source-literal range for `Int`.

Binary and hexadecimal literals are not fixed-width bit patterns. In
particular, a hexadecimal spelling with its high bit set does not become a
negative value, and leading zeroes do not give the value a width. A value
above the existing positive `Int` limit is an out-of-range literal even if it
contains no more than 64 binary digits or 16 hexadecimal digits.

A leading `-` remains the existing unary negation operator and is not part of
the literal token. This proposal does not change the range or edge cases of
negative decimal literals.

Equivalent spellings compare and match by value:

```veln
0b1010 == 10
0x0a == 10

match value
  0x0a -> "ten"
  _ -> "other"
end
```

The three spellings lower to the same canonical `Int` value before constant
evaluation and backend emission. Backends must not parse source spelling with
host-language defaults or reinterpret a hexadecimal value as unsigned
two's-complement data.

## Language Surface

Binary and hexadecimal spellings are accepted anywhere the language expects
an ordinary nonnegative integer literal, including:

- expression literals
- literal patterns and record-field literal patterns
- binary schema reserved-value arguments
- schema dispatch tags and other schema constraints defined in terms of an
  integer literal
- compile-time checks that require a literal integer operand

Decimal widths embedded in schema primitive names, such as the `16` in
`uint16be`, remain decimal name components. This proposal does not add forms
such as `uint0x10be`.

All consumers that inspect an integer literal must use one shared radix-aware
conversion rule. A consumer must not accept a prefixed spelling syntactically
and then silently treat it as zero, reject it as a decimal-only value, or
compare its source text instead of its value.

## Formatting And Editor Support

The formatter preserves the accepted literal spelling, including the chosen
radix, leading zeroes, and hexadecimal digit case. It does not rewrite
`0b1010` or `0x0A` to decimal and does not choose a canonical case for
hexadecimal digits.

Editor semantic tokens classify all three forms as integer numbers. Syntax
highlighting, hover information, documentation rendering, repair previews,
and other source-aware tools treat the prefix and digits as one literal.

## Diagnostics

A prefixed integer with no digits reports that the selected radix requires at
least one digit. A candidate containing an invalid digit reports that the
specific character is not valid for the selected radix at that character's
span. Related context names the accepted digit set when that helps repair the
source.

An otherwise well-formed literal whose mathematical value is outside the
existing `Int` literal range reports the failed range check at the complete
literal span. The diagnostic identifies the radix and accepted maximum; it
does not suggest truncation, wrapping, or reinterpretation as a negative
value.

Malformed prefixed literals produce one lexical or parse diagnostic and do
not cascade into unresolved-name or adjacent-expression diagnostics caused by
splitting the candidate.

## Non-Goals

- Adding octal literals or uppercase `0B` and `0X` prefixes.
- Adding digit separators.
- Adding hexadecimal or binary floating-point literals.
- Adding unsigned integers, width-suffixed integers, or arbitrary-precision
  integers.
- Treating a literal as a fixed-width bit pattern based on its digit count.
- Changing decimal literal syntax, `Int` width, overflow behavior, or unary
  negation semantics.
- Rewriting integer literals between radices during formatting.

## Completion Criteria

- The lexer and parser accept the proposed grammar and reject missing digits,
  invalid digits, unsupported prefixes, separators, and prefixed float forms
  without token-splitting cascades.
- AST and lowered representations either retain the source spelling where
  tooling needs it or carry an explicit normalized value; semantic consumers
  do not depend on decimal-only string parsing.
- Type analysis, literal patterns, constant evaluation, contract analysis,
  repair reasoning, schema arguments and constraints, IR, and every backend
  agree on the value and range of each accepted spelling.
- The formatter preserves radix and digit spelling, and editor support
  classifies each complete literal consistently.
- Focused human and structured diagnostic coverage exists for a missing digit,
  an invalid binary digit, an invalid hexadecimal digit, an unsupported
  uppercase prefix, a separator, a prefixed float form, and an out-of-range
  value.
- Executable specification examples cover equivalent decimal, binary, and
  hexadecimal values in expressions and patterns, plus representative schema
  literal positions.
- Current behavior is documented under `../../specification/` and executable
  evidence is added under `../../../examples/specification/` before this proposal
  leaves the active catalog.
- The completed proposal record is archived under this implemented-proposal
  directory, and the proposal is removed from the
  active catalog when all completion criteria are satisfied.
