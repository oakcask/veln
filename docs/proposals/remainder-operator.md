---
role: proposal
update-when: Integer remainder contract typing, contract execution, constant reasoning, or repair analysis is implemented or revised.
---

# Integer remainder in contracts and repair

Ordinary expressions implement signed integer `%`, including parsing,
formatting, integer-only typing, JVM execution, zero-divisor diagnostics, and
public run output. The current behavior belongs to the
[source-surface](../specification/source-surface.md),
[operator](../specification/types.md#operators-and-diagnostics), and
[run JSON](../specification/run-json.md) specifications.

This remaining proposal extends the same operator into function and test
contract predicates and into contract reasoning. It does not change ordinary
expression behavior.

## Remaining scope

Contract predicates in `require`, `ensure`, `invariant`, and `satisfy` do not
currently accept `%`. The remaining work must add all of these behaviors
together:

- Parse and type `%` in contract predicates as an `Int`-only binary operator
  with the ordinary multiply-level precedence and left associativity.
- Execute runtime-checked contract predicates with the ordinary signed
  remainder model and zero-divisor failure behavior.
- Evaluate literal-only `%` subexpressions during static reasoning when both
  signed 64-bit operand values are known and the divisor is nonzero.
- Leave nonliteral remainder predicates runtime-checked. Do not infer modular
  repairs or claim that an unresolved congruence is proved.

Floating-point remainder, Euclidean modulo, operator overloading, `%=` and the
separate schema count, length, and validation expression languages remain out
of scope.

## Numeric and failure contract

For signed integers `a` and nonzero `b`, contract execution and constant
reasoning must use `q = trunc(a / b)` and `r = a - q * b` over mathematical
integers. The result is `r`. Minimum `Int` remainder by `-1` is `0` and must
not overflow. Literal and dynamic zero divisors must follow the ordinary
operator's diagnostic contract at the applicable contract source location.

Contract typing must not reuse `/`'s exact rational reasoning. A surrounding
`Float` context must not select floating-point remainder or widen either
operand.

## Acceptance model and planned evidence

| Concern | Input or condition | Required observation | Planned verification boundary |
| --- | --- | --- | --- |
| Contract numeric results | Literal signed, endpoint, and large-divisor `%` cases in `require`, `ensure`, and `invariant` | Static conclusions and runtime checks agree with ordinary execution, including minimum `Int % -1 == 0` | Contract reasoning and backend contract tests |
| Dynamic predicate | `value % 2 == 0` | Even input passes and odd input reports the existing contract failure at the predicate | Contract execution tests and CLI fixtures |
| Contract types | Float, Boolean, or String operand | Operand reports `type.mismatch`; the result remains `Int` | Semantic diagnostic tests |
| Contract zero divisor | Literal zero and a runtime-computed zero | Literal failure is static; computed failure uses the ordinary runtime remainder diagnostic without a host exception | Diagnostic and run fixtures |
| Satisfy | A `%` predicate over a hole or supplied value | The predicate is typed and remains runtime-checked when values are not statically known | Satisfy analysis tests |
| Repair | Nonliteral `value % 3 == 1` | No modular repair or false proof is produced; independent supported constraints may retain their own repairs | Repair analysis tests |

Expected numeric values must come from explicit values or an independent
arbitrary-precision truncating-division model. Tests must not use the backend
remainder helper as their oracle.

## Completion route

Add executable contract evidence before updating
[contracts](../specification/contracts.md). Audit contract parser dispatch,
predicate typing, static numeric reasoning, backend contract emission, and
repair normalization. When every row above has executable coverage, remove
this proposal and its Ready catalog entry.

## Related work and alternatives

- [Java SE Language Specification, integer remainder](https://docs.oracle.com/javase/specs/jls/se21/html/jls-15.html#jls-15.17.3)
  defines dividend-signed remainder and zero for minimum integer remainder by
  `-1`. Veln uses that numeric result but owns its diagnostic contract.
- [The Rust Reference, arithmetic operators](https://doc.rust-lang.org/reference/expressions/operator-expr.html#arithmetic-and-logical-binary-operators)
  also describes truncating signed remainder, but Rust classifies minimum
  signed integer `% -1` as overflow. Veln must preserve the representable
  mathematical result instead.
- Euclidean modulo would change negative-dividend results. A separately named
  helper can provide that operation without giving `%` two meanings.
