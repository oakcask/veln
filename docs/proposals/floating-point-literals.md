---
role: proposal
update-when: The proposed decimal-exponent or hexadecimal Float grammar, shared conversion and range policy, special-value token decision, or literal acceptance evidence changes.
---

# Floating-point Exponent Literals

Add decimal exponent notation and binary-scaled hexadecimal notation for finite
`Float` values. Keep NaN and infinity available through the proposed
standard-library constants. This
is Design In Progress work, separate from delivering those named constants.

## Decision and scope

Treat finite exponent notation and special-value literals as separate
decisions. Introduce decimal and hexadecimal exponent notation together, as
one language extension alongside [numeric constants](numeric-constants.md).
Keep NaN and infinities as standard-library values. The exponent extension is
not a prerequisite for publishing those constants. The rules below are the
proposed acceptance model, not current syntax.

| Candidate | Decision | User benefit and cost |
| --- | --- | --- |
| Decimal significand with decimal exponent, such as `1.25e-3` | Recommend together with hexadecimal notation | Supports scientific notation and compact magnitude scaling with the same conversion, rounding, and error contract. |
| Hexadecimal significand with binary exponent, such as `0x1.8p+1` | Recommend | Expresses arbitrary finite binary64 values and rounding boundaries directly; requires complete lexical, conversion, diagnostic, and tooling support. |
| Dedicated `nan`, `inf`, or `infinity` tokens | Defer | Avoids an import but duplicates named values. Adds literal and identifier rules without expressing NaN sign or payload. |
| Overflowing finite literal as an infinity spelling | Reject | A mistyped exponent must not silently manufacture infinity. |
| Raw-bit NaN literal or payload syntax | Defer | Useful for binary interoperability only after a separate payload-preservation and bit-conversion contract exists. |

This recommendation follows from source expressiveness, not an assumption that
decimal notation is inherently inaccurate. A correctly rounded decimal parser
can also express every finite binary64 value. Hexadecimal notation makes binary
structure and exponent boundaries explicit without long decimal expansions.
For example, the proposed source spellings for the positive minimum, positive
normal minimum, epsilon, and maximum are respectively `0x1p-1074`,
`0x1p-1022`, `0x1p-52`, and `0x1.fffffffffffffp1023`.

At investigation revision `d83fb79b4`, `crates/veln-syntax/src/lexer.rs` recognizes
decimal floats only when a decimal point is followed by digits. It does not
consume decimal exponents. Its prefixed-number path and
`crates/veln-literals/src/lib.rs` validate integers and reject prefixed floats.
The hexadecimal extension must deliberately replace that rejection for its
accepted grammar; it is not enabled by the host's floating-point parser.
Both decimal exponent recognition and hexadecimal float recognition are
explicit scope changes. Existing dotted decimal floats also adopt the shared
conversion and range contract below; values that previously rounded silently
to infinity or zero may become errors.

### Proposed decimal source contract

Keep the existing dotted decimal shape with digits on both sides of the dot.
Add an optional `e` or `E` exponent to that shape and allow integer-shaped
significands when an exponent is present. Exponent digits are decimal and an
optional exponent sign is part of the literal. A leading minus remains a unary
operator. Every exponent form has type `Float`; `1e0` does not become an `Int`.

```text
DecimalFloat    ::= Digit+ "." Digit+ DecimalExponent? | Digit+ DecimalExponent
DecimalExponent ::= ("e" | "E") ("+" | "-")? Digit+
```

A decimal exponent scales the significand by ten to that exponent. Accept
`1e3`, `1E+3`, `1e-3`, and `1.25e2`. Plain `123` remains an `Int` and `1.25`
remains a `Float`. Do not add leading-dot or trailing-dot decimal forms,
separators, or suffixes as part of this extension. In particular, `.5e1` and
`1.e3` are not newly accepted decimal literals. An adjacent `+` or `-` is part
of the exponent only immediately after its marker; `1e2+3.0` is addition.

### Proposed hexadecimal source contract

Use a lowercase `0x` prefix, at least one hexadecimal significand digit, an
optional radix point, and a mandatory `p` or `P` exponent marker. The exponent
has an optional sign and at least one decimal digit. The exponent scales the
significand by a power of two, not a power of sixteen. Hexadecimal digits may
use either case. Do not add separators, type suffixes, or uppercase `0X` as
part of this extension. A leading minus remains a unary operator.

```text
HexFloat       ::= "0x" HexSignificand ("p" | "P") ("+" | "-")? Digit+
HexSignificand ::= HexDigit+ ("." HexDigit*)? | "." HexDigit+
Digit          ::= "0" ... "9"
HexDigit       ::= Digit | "a" ... "f" | "A" ... "F"
```

Every accepted spelling has type `Float`, even when its value is integral.
Without a radix point or exponent marker, the existing hexadecimal integer
rules still apply: `0x1e2` is an `Int`. In `0x1e+2`, `e` is an integer digit
and `+` is an operator, not a decimal exponent. A radix point without the
required binary exponent is invalid. Malformed hexadecimal float candidates
must produce a focused literal error rather than become an unrelated integer
and identifier sequence. Lexer tests must fix token boundaries for missing
exponent digits and adjacent operators.

### Shared conversion and tooling contract

For both decimal and hexadecimal literals, interpret the significand and
exponent as an exact mathematical value, then round once to binary64 using
round to nearest, ties to even. Do not round the
significand to binary64 before applying the exponent. Reject a nonzero literal
when its rounded result is infinity or zero. Accept nonzero subnormal results,
including a rounded result equal to the smallest positive value. A zero
significand remains zero regardless of exponent, and unary negation preserves
negative zero. These rules also apply to existing dotted decimal literals.
Decimal conversion must not first round the significand or an intermediate
power of ten; hexadecimal conversion must not inherit decimal parser rules.
A zero significand with a syntactically valid exponent remains valid even when
the exponent magnitude exceeds a host integer range.

Accept both new forms in ordinary expressions and existing Float-literal
pattern positions. Module initializers also require the
[scalar-data extension](numeric-constants.md#construction-and-implementation-boundary).
Patterns keep existing Float matching semantics; the notation does not
introduce total ordering or bitwise equality. Formatting must preserve valid
literal spelling and keep unary operators separate. Editor tokenization and
diagnostics must recognize the complete accepted or malformed candidate.

| Source case | Required observation | Planned verification |
| --- | --- | --- |
| `1e3`, `1E+3`, `1.25e2`, `1e0` | Float values 1000, 1000, 125, and 1 respectively | Lexer/parser fixtures and independent backend vectors |
| `9007199254740993e0` | Ties-to-even result 2^53, bits `4340000000000000` | Decimal conversion unit tests |
| `1e309`, `1e-400`, `5e-324` | Overflow error, nonzero-to-zero error, and least positive subnormal respectively | Conversion, diagnostics, and backend bit checks |
| `0e9999999999999999999999999`, `-0.0e-9999999999999999999999999` | Positive and negative zero respectively, without host exponent overflow | Conversion and raw-bit checks |
| `1e`, `1e+`, `1.2e-`, `1e2f`, `1_e2`, `1.e3`, `.5e1` | Invalid or unsupported decimal literal forms; no interpretation as an exponent Float followed by a suffix | Lexical recovery, diagnostics, and formatter fixtures |
| `1e2+3.0`, `1.25`, `123` | Addition; existing dotted Float; existing Int | Token-boundary and type regression fixtures |
| `0x1.8p+1`, `0x.8P1`, `0x1.p0` | Float values 3, 1, and 1 respectively | Lexer/parser fixtures and independent backend bit vectors |
| `0x1p-1074`, `0x1p-1022`, `0x1p-52`, `0x1.fffffffffffffp1023` | Exact bits from the [constant table](numeric-constants.md#floating-point-values) | Shared literal-conversion unit tests and JVM execution checks |
| `0x1.00000000000008p0` | Halfway tie rounds to 1, bits `3ff0000000000000` | Independent ties-to-even vectors |
| `0x1.00000000000018p0` | Halfway tie rounds to bits `3ff0000000000002` | Independent ties-to-even vectors |
| `0x1p1024`, `0x1p-1075` | Overflow and nonzero-to-zero underflow errors respectively | Checked diagnostics with the failed literal span |
| `0x1.8p-1075` | Rounds to the least positive subnormal | Conversion and backend bit checks |
| `0x0p0`, `-0x0p0` | Positive and negative zero encodings respectively | Backend raw-bit checks |
| `0x1p+`, `0x.p1`, `0x1.2`, `0X1p0`, `0x1_p0` | Rejected as malformed or unsupported literal spellings | Lexical recovery, diagnostics, and formatter rejection fixtures |
| `0x1e2`, `0x1e+2` | Existing hexadecimal integer and integer-addition interpretation | Integer parser regression fixtures |
| Unbound `nan`, `inf`, `infinity` | Ordinary name resolution, not built-in literals | Name-resolution cases, including legal local bindings with those names |

These are compiler and backend contracts. Their toolchain cases complement
the companion tests for the named standard-library values.

### Why special values remain named values

The existing lowercase identifier rules allow names such as `nan` and `inf`.
Backward compatibility is not required, but reserving these names still adds
language rules that need a user benefit. Qualified standard-library values
already express the intended special values and fit existing navigation and
documentation. A NaN token would also need an explicit decision about literal
patterns, where ordinary NaN equality cannot provide a successful match.

Do not introduce contextual special tokens that change meaning depending on
whether a binding exists. If non-finite source interchange or import-free
numeric data later becomes a concrete requirement, reconsider dedicated
tokens with explicit identifier, sign, pattern, and serialization boundaries.
Hexadecimal finite literals alone cannot represent infinity or NaN, and neither
literal decision solves the signed integer lower-bound construction problem.

## Related work

The following primary specifications were inspected directly. They support
the design choices, not claims of current Veln conformance.

- [Java Language Specification, floating-point literals](https://docs.oracle.com/javase/specs/jls/se21/html/jls-3.html#jls-3.10.2)
  specifies hexadecimal significands with mandatory binary exponents and
  rejects literal conversion to infinity or nonzero-to-zero underflow. It
  supplies non-finite values through predefined fields. Adopt that separation
  and range-error policy for both exponent surfaces, not Java's suffixes,
  separators, or uppercase prefix. This is a design choice for Veln; JVM
  execution alone does not establish source-language conversion behavior.
- [Go specification, floating-point literals](https://go.dev/ref/spec#Floating-point_literals)
  requires `p`/`P` for hexadecimal exponents and uses decimal exponent digits
  to scale by powers of two. It also explicitly distinguishes hexadecimal
  integer arithmetic from exponent notation. Adopt those lexical distinctions.
  Its decimal `e`/`E` notation also informs the proposed decimal extension,
  but Veln retains digits on both sides of a decimal radix point.
  Go's untyped arbitrary-precision constant model does not transfer to Veln's
  directly typed binary64 values; Veln needs its own rounding and range gates.

## Readiness and completion

Promote this proposal to Ready only after the literal acceptance table has
fixed executable fixtures and an end-to-end conversion prototype.
All consumers must use the same validated value: ordinary expression emission,
patterns, module data, and contract evaluation must not independently parse
hex text as decimal, round decimal scaling twice, or fall back to zero on
failure. The current JVM emission
sites in `function_flow.rs` and `pattern_and_value_emission.rs` under
`crates/veln-backend-jvm/src/classfile/` parse float text as `f64` and use zero
on parse failure; forwarding new token text unchanged is insufficient.

Before completion, update the current source-surface, type, execution, and
editor specifications with checked examples. Remove this proposal and its
catalog entry once both selected exponent surfaces are implemented. Deferred
special-token alternatives do not keep completed work open. This proposal
changes no current specifications or executable examples.
