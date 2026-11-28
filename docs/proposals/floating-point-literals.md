---
role: proposal
update-when: The proposed decimal-exponent or hexadecimal Float grammar, token boundaries, tooling acceptance, current dotted-decimal conversion boundary, or readiness classification changes.
---

# Floating-point Exponent Literals

Add decimal exponent notation and binary-scaled hexadecimal notation for
finite `Float` values. Keep NaN and infinity available through the separate
proposed standard-library constants. The shared conversion and range behavior
for existing dotted-decimal literals is implemented and belongs to the
[source-surface specification](../specification/source-surface.md#float-literals).

## Decision and scope

Introduce decimal and hexadecimal exponent notation together as one language
extension. The extension reuses the implemented rule that converts an exact
source value once to binary64 with round-to-nearest, ties-to-even, and rejects
a nonzero value that rounds to infinity or zero. It extends that boundary with
new grammars; it does not redesign existing dotted-decimal conversion.

| Candidate | Decision | User benefit and cost |
| --- | --- | --- |
| Decimal significand with decimal exponent, such as `1.25e-3` | Recommend together with hexadecimal notation | Supports scientific notation and compact magnitude scaling. |
| Hexadecimal significand with binary exponent, such as `0x1.8p+1` | Recommend | Expresses binary64 values and rounding boundaries directly; requires complete lexical and tooling support. |
| Dedicated `nan`, `inf`, or `infinity` tokens | Defer | Avoids an import but duplicates named values and introduces identifier rules. |
| Overflowing finite literal as an infinity spelling | Reject | A mistyped exponent must not silently manufacture infinity. |
| Raw-bit NaN literal or payload syntax | Defer | Requires a separate payload-preservation and bit-conversion contract. |

Correctly rounded decimal conversion can express every finite binary64 value.
Hexadecimal notation makes binary structure and exponent boundaries explicit.

### Proposed decimal source contract

Keep the existing dotted decimal shape with digits on both sides of the point.
Add an optional `e` or `E` exponent and allow integer-shaped significands
when an exponent is present. Exponent digits are decimal and an optional sign
after the exponent marker is part of the literal. A leading minus remains a
unary operator. Every exponent form has type `Float`.

```text
DecimalFloat    ::= Digit+ "." Digit+ DecimalExponent? | Digit+ DecimalExponent
DecimalExponent ::= ("e" | "E") ("+" | "-")? Digit+
```

Accept `1e3`, `1E+3`, `1e-3`, and `1.25e2`. Plain `123` remains an
`Int` and `1.25` remains an existing dotted-decimal `Float`. Do not add
leading-dot or trailing-dot forms, separators, or suffixes. `1e2+3.0` is
addition.

### Proposed hexadecimal source contract

Use a lowercase `0x` prefix, at least one hexadecimal significand digit, an
optional radix point, and a mandatory `p` or `P` exponent marker. The
decimal exponent scales the significand by a power of two. Hexadecimal digits
may use either case. Do not add separators, suffixes, or uppercase `0X`. A
leading minus remains unary negation.

```text
HexFloat       ::= "0x" HexSignificand ("p" | "P") ("+" | "-")? Digit+
HexSignificand ::= HexDigit+ ("." HexDigit*)? | "." HexDigit+
Digit          ::= "0" ... "9"
HexDigit       ::= Digit | "a" ... "f" | "A" ... "F"
```

Every accepted spelling has type `Float`. Without `p` or `P`, `0x1e2`
remains an `Int`, and `0x1e+2` remains integer addition. A radix point
without the mandatory binary exponent is invalid. Malformed hexadecimal
candidates must produce a focused literal error rather than unrelated tokens.

### Conversion and tooling extension

Both exponent forms must supply their exact mathematical value to the existing
shared Float conversion boundary. Decimal conversion must not round a
significand or intermediate power of ten first. Hexadecimal conversion must
not inherit decimal parser rules. A syntactically valid zero significand stays
zero even when the exponent magnitude exceeds a host integer range. Negating
such a zero produces negative zero.

Accept both forms in ordinary expressions and Float-literal patterns. Float
module initializers remain dependent on the separate
[scalar-data extension](numeric-constants.md#construction-and-implementation-boundary).
Formatting must preserve valid spelling and keep unary operators separate.
Editor tokenization and diagnostics must recognize each complete accepted or
malformed candidate.

| Source case | Required observation | Planned verification |
| --- | --- | --- |
| `1e3`, `1E+3`, `1.25e2`, `1e0` | Float values 1000, 1000, 125, and 1 | Lexer/parser fixtures and backend vectors |
| `9007199254740993e0` | Ties-to-even bits `4340000000000000` | Shared conversion tests |
| `1e309`, `1e-400`, `5e-324` | Overflow error, zero-underflow error, and least positive subnormal | Diagnostics and backend bit checks |
| `0e9999999999999999999999999`, `-0.0e-9999999999999999999999999` | Positive and negative zero without host exponent overflow | Conversion and raw-bit checks |
| `1e`, `1e+`, `1.2e-`, `1e2f`, `1_e2`, `1.e3`, `.5e1` | Rejected forms with stable token recovery | Lexer, diagnostics, and formatter fixtures |
| `1e2+3.0`, `1.25`, `123` | Addition, existing dotted Float, and existing Int | Token-boundary and type regressions |
| `0x1.8p+1`, `0x.8P1`, `0x1.p0` | Float values 3, 1, and 1 | Lexer/parser and backend vectors |
| `0x1p-1074`, `0x1p-1022`, `0x1p-52`, `0x1.fffffffffffffp1023` | Exact binary64 boundary bits | Shared conversion and JVM checks |
| `0x1.00000000000008p0`, `0x1.00000000000018p0` | Halfway ties round to even | Independent bit vectors |
| `0x1p1024`, `0x1p-1075`, `0x1.8p-1075` | Overflow, zero underflow, and least-subnormal rounding | Diagnostics and backend bit checks |
| `0x0p0`, `-0x0p0` | Positive and negative zero encodings | Backend raw-bit checks |
| `0x1p+`, `0x.p1`, `0x1.2`, `0X1p0`, `0x1_p0` | Rejected malformed or unsupported spellings | Lexer, diagnostics, and formatter fixtures |
| `0x1e2`, `0x1e+2` | Existing hexadecimal integer and addition behavior | Integer regression fixtures |

NaN and infinities remain named-value work. Unbound `nan`, `inf`, and
`infinity` continue through ordinary name resolution.

## Related work

- [Java Language Specification, floating-point literals](https://docs.oracle.com/javase/specs/jls/se21/html/jls-3.html#jls-3.10.2)
  specifies hexadecimal significands with mandatory binary exponents and
  rejects conversion to infinity or nonzero-to-zero underflow. Veln adopts
  that separation and range policy, but not Java suffixes, separators, or
  uppercase prefixes. JVM execution does not establish Veln syntax.
- [Go specification, floating-point literals](https://go.dev/ref/spec#Floating-point_literals)
  requires `p` or `P` for hexadecimal exponents and distinguishes
  hexadecimal integer arithmetic from exponent notation. Veln adopts those
  lexical distinctions while retaining digits on both sides of a decimal
  point. Go's untyped constants do not transfer to Veln's directly typed
  binary64 values.

## Readiness and completion

The grammar, token boundaries, conversion extension, and acceptance table fix
the remaining implementation contract. The current dotted-decimal conversion
API and its expression, pattern, contract, diagnostic, formatter, editor, and
JVM coverage are prerequisites available to the exponent implementation, not
future deliverables of this proposal.

Completion requires both exponent surfaces to use that boundary in ordinary
expressions, patterns, contracts, formatting, editor features, and JVM
execution. Future Float module data must also use the boundary when the
separate scalar-data extension adds that consumer, but it is not a completion
dependency for this proposal. Update the source-surface, type, execution, and
editor specifications with checked exponent examples, then remove this
proposal and its catalog entry. Deferred special-token alternatives do not
keep completed exponent work open.
