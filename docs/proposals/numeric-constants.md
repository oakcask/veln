---
role: proposal
update-when: The proposed std::num constants, literal construction dependency, scalar module-value representation, numeric acceptance gates, or readiness classification changes.
---

# Standard-library Numeric Constants

Expose the principal bounds and special values of `Int` and `Float` through
typed immutable standard-library values. Users should not need to copy large
decimal literals or manufacture infinity through arithmetic. This is planned
behavior and a Ready implementation target. The public values, scalar-data
admission boundary, and comparison outcomes below define its contract;
construction and comparison tests are implementation acceptance.

## Current boundary and purpose

Veln already has typed public module values and package-aware imports; see
[module declarations](../specification/source-surface.md#module-immutable-value-declarations)
and [name resolution](../specification/name-resolution.md#namespaces-and-use-roles).
Those mechanisms supply the intended public surface without a new `const`
keyword, uppercase value names, associated constants, or implicit prelude names.

At investigation revision `d83fb79b4`,
`crates/veln-sema/src/analysis/module_values/admission.rs` admits integer
literals and integer literal negation, but not floating-point literals or
arbitrary arithmetic in module initializers. The source literal range ends at
`9223372036854775807`; negating that literal cannot express `Int`'s lower bound.
The JVM backend uses boxed JVM longs and doubles, and parses floating-point
literal text as Rust `f64` in
`crates/veln-backend-jvm/src/classfile/function_flow.rs`.
[Module-value execution](../specification/execution.md#runtime-readiness-and-host-boundaries)
currently documents a data surface without floating-point leaves. A library
file containing decimal constants alone therefore cannot deliver this proposal.

The first delivery covers the existing signed 64-bit `Int` and binary64
`Float`. These are language-value limits, independent of pointer size or host
architecture. Schema field widths do not introduce additional numeric types.

## Proposed public API

Add `std::num::int` and `std::num::float`. The planned source layout is:

```text
crates/veln-stdlib/veln/num/
  int.veln
  int.test.veln
  float.veln
  float.test.veln
```

Each listed name is a `pub let` with
the declared scalar type. Reads have no effects and require no parentheses.
The modules follow ordinary import, visibility, shadowing, documentation, and
navigation rules. Importing both modules can make bare `min` and `max`
ambiguous; qualified reads select the intended declaration.

Illustrative future usage, not an executable example of current behavior:

```veln
use std::num::int
use std::num::float

fn integer_upper_bound() -> Int
	int::max
end

fn initial_distance() -> Float
	float::infinity
end
```

### Integer values

| Name | Type | Required value |
| --- | --- | --- |
| `int::min` | `Int` | −2^63 = −9223372036854775808 |
| `int::max` | `Int` | 2^63 − 1 = 9223372036854775807 |

These are inclusive bounds. `Int` has no infinity or NaN. This API does not
change integer overflow, division, shifts, or source literal parsing.

### Floating-point values

The bit column describes the binary64 encoding as a mathematical unsigned
64-bit word. It is acceptance data, not proposed Veln integer-literal syntax.
All entries have type `Float`.

| Name | Meaning | Required value or encoding |
| --- | --- | --- |
| `float::min` | Most negative finite value | −(2 − 2^−52) × 2^1023; bits `ffefffffffffffff` |
| `float::max` | Largest finite value | (2 − 2^−52) × 2^1023; bits `7fefffffffffffff` |
| `float::min_positive_normal` | Smallest positive normal value | 2^−1022; bits `0010000000000000` |
| `float::min_positive` | Smallest positive nonzero value, including subnormals | 2^−1074; bits `0000000000000001` |
| `float::epsilon` | Gap from 1 to the next greater representable value | 2^−52; bits `3cb0000000000000` |
| `float::infinity` | Positive infinity | Bits `7ff0000000000000` |
| `float::neg_infinity` | Negative infinity | Bits `fff0000000000000` |
| `float::nan` | A quiet NaN | All exponent bits set, nonzero fraction, quiet bit set; sign and remaining payload unspecified |

`min` always denotes the lower finite bound, for both numeric types. Neither
infinity is a finite bound. The two positive minima have different names so
normal-only algorithms can distinguish their domain from gradual underflow.
`epsilon` is spacing at 1, not a universal absolute tolerance and not the
smallest positive value. Examples and generated documentation must explain
these distinctions.

Signed zero remains expressible through existing numeric expressions; this
proposal adds no zero or one aliases. Mathematical constants such as pi and e,
radix/precision/exponent metadata, safe integer conversion limits, additional
numeric widths, and a public bit-conversion API are outside this delivery.

### Special-value observations

For an ordinary finite `Float` `x`,
`float::neg_infinity < x` and `x < float::infinity` are true.
Positive and negative infinity compare equal to themselves and unequal to
each other. For `n = float::nan`, `n == n` is false and `n != n` is true.
Equality with any other float is false; every ordered comparison involving
NaN is false, including comparison with itself or infinity. These rules apply
to ordinary scalar operators, not structural container equality or sorting.
They are delivery requirements to audit against current runtime behavior,
not claims that all current operators already satisfy them.

A NaN read must not throw or require handling an effect. NaN sign, payload,
object identity, and propagation through arithmetic are not guaranteed.
Public classification functions are not a prerequisite: `x != x` can detect
NaN under the scalar comparison contract above. Text formatting, parsing,
serialization, total ordering, and non-finite arithmetic policy are separate
contracts; publishing constants must not silently redefine them.

## Literal policy

[Floating-point Exponent Literals](floating-point-literals.md) recommends
an independently deliverable decimal and hexadecimal exponent extension and
defers NaN and infinity tokens. Named standard-library values remain the public special-value
surface. Exponent literals are not a prerequisite for this constants API;
when available, they make finite constant definitions and boundary fixtures
clearer. Neither decision solves construction of the signed integer lower bound.

## Construction and implementation boundary

Keep public values as ordinary source declarations with explicit annotations.
Extend checked core, typed IR, and module-value materialization to carry
floating-point scalar data, including non-finite values, without evaluating
user code at startup. Admit ordinary floating-point literals and literal
negation in data initializers as part of that extension. Preserve the existing
reference identity, dependency, and cycle rules for aliases and nested data.

When the exponent-literal extension is implemented, prefer its exact source
literals for finite float constants. Until then, a library delivery may use
the same internal scalar-data bridge as the special values. The recommended
bridge is a fixed set of compiler-known scalar data leaves for the standard
package, referenced by private bindings in the two source modules. It must
represent the full `Int` lower bound and required special float values directly,
and finite values when the literal route is unavailable. It must not depend
on out-of-range literal text, decimal overflow, division by zero, general constant evaluation,
or runtime host calls. It must use trusted standard-package identity; a
same-spelled local package or module cannot acquire compiler privileges.
Bridge spellings are internal and are not alternative supported public names.

The exact private bridge declaration and registration representation is an
implementation choice within this fixed admission boundary: only trusted
standard-package scalar leaves receive the privilege, ordinary Float literal
data and negation become admissible, and general user calls or effectful
initializers remain rejected. Implement the package-loading, resolution,
core/IR, and JVM path together and test both admitted and rejected origins.
No general new declaration facility is part of this target; if one becomes
necessary, resolve that scope change in a separate proposal. Do not replace
the specified public values with getters to avoid implementing scalar data.

For NaN, the implementation may select a convenient quiet encoding. All
finite constants and infinities must preserve the specified bits through
module-value aliases, records, and function returns. A future backend must
meet the same public contract rather than substitute its host's native limits.

## Related work and alternatives

The following inspected primary documentation supports the design rationale;
it does not prove Veln conformance.

- [Rust primitive `f64` constants](https://doc.rust-lang.org/std/primitive.f64.html#associatedconstant.MIN)
  define `MIN` as the most negative finite value, `MIN_POSITIVE` as the least
  positive normal value, and `EPSILON` as the gap above 1. The
  [NaN contract](https://doc.rust-lang.org/std/primitive.f64.html#associatedconstant.NAN)
  also warns against relying on a particular NaN bit pattern. Adopt the signed
  finite bound and epsilon meanings and the unspecified NaN payload. Veln has
  ordinary lowercase module values rather than Rust associated constants.
  Adapt the positive-minimum names to expose both normal and subnormal bounds
  explicitly; copying Rust's `MIN_POSITIVE` meaning would hide that difference.
- [Java SE 21 `Double` fields](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/lang/Double.html#MIN_VALUE)
  distinguish `MIN_VALUE` (the least positive nonzero value) from
  [`MIN_NORMAL`](https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/lang/Double.html#MIN_NORMAL).
  The fields provide binary64 formulas and encodings for the positive minima
  and maximum. Adopt these as independent acceptance vectors, but reject the
  name `min_value` for a positive float: integer minimum and float minimum
  would otherwise have different meanings. Java's boxed `Double.equals`
  differs from scalar NaN equality; only the documented primitive comparison
  rules inform the proposed scalar contract.

Other considered choices:

| Alternative | Decision and reason |
| --- | --- |
| `Int::MAX` / `Float::INFINITY` | Defer associated-constant lookup and uppercase value rules; existing module values already provide typed immutable reads. |
| Pure nullary getters | Reject for this API: users would call functions for immutable data, and getters would not supply module-data initializer references. |
| Global prelude constants | Reject: short names collide and obscure which type's bound is selected. |
| One flat module with `int_min`, `float_min`, etc. | Prefer type-specific modules so bounds and special values are discoverable together and use consistent local names. |
| Source arithmetic or exceptional literal strings | Reject as a construction contract: admission and arithmetic failure policy would determine whether a constant exists. |
| Host-dependent bounds | Reject: the supported scalar model fixes these values; pointer width is irrelevant. |

## Acceptance model and verification plan

These are planned checks, not implemented or passing evidence. The formulas
and bit vectors above are the acceptance authority. Assertions must not derive
expected values from the same constant factory being tested. Internal host
tests may inspect raw bits without adding a public bit-conversion API.

| Case | Observable acceptance condition | Intended evidence |
| --- | --- | --- |
| Integer bounds | Both values equal the signed 64-bit endpoints, with exact `Int` type | Integer-module companion tests; independent literal/host vectors for the lower endpoint |
| Finite float values | Each of the five finite constants has exactly the listed bits; both positive minima remain nonzero and distinct | Float-module companion arithmetic/order tests and focused JVM materialization bit checks |
| Epsilon | `1.0 + epsilon > 1.0`; the representable binary64 gap has bits `3cb0000000000000` | Float companion tests and independent bit inspection |
| Infinities | Signs, exact encodings, self-equality, and ordering against finite extrema match the contract | Float companion tests; JVM bit checks |
| NaN | The value is quiet NaN; self-inequality and all specified scalar equality/order outcomes hold for NaN on either side | Float companion tests; quiet-bit check in backend tests, without asserting payload or sign |
| Reads and forwarding | Pure functions and annotated module values can read each constant; aliases and nested records preserve bits | Companion API tests plus focused compiler/core/IR and JVM tests for scalar-data preservation |
| Invalid use | Calling a constant fails as a non-callable value; assigning it to the other scalar type fails | Checked specification diagnostic examples exercising the existing type contracts |
| Import and origin boundary | Qualified and aliased imports select the standard declarations; local names obey ordinary shadowing; spoofed bridge origins receive no privilege | Focused package-loading and name-resolution tests, including cached standard environments |
| Unsupported initializer | General user calls and effectful initialization remain rejected | Existing admission tests extended for the narrow new data surface and bridge |
| Discovery | Generated stdlib docs list each public value with its type and meaning; definition navigation reaches its source declaration | Focused documentation and language-service integration cases |

Standard-library API tests belong under `crates/veln-stdlib/veln/` in companion
files, following the
[test placement policy](../reference/toolchain-test-harness.md).
Toolchain cases must identify an independent compiler, backend, CLI, or
integration contract from the table; they must not replace companion API
coverage. No fake host effects are needed for these pure values.

## Readiness and completion

Ready selection uses the public value and comparison tables and the narrow
construction boundary above. No prototype or passing numeric API suite is
required before implementation. Private scalar representation does not change
those public decisions, and any existing equality or ordering mismatch is an
implementation defect to fix against the specified non-finite rows.

During implementation:

1. Verify the narrow construction bridge, including `Int`'s lower bound,
   subnormals, quiet NaN, and package-origin rejection.
2. Audit and correct scalar equality and ordering against the non-finite rows;
   do not infer correctness from JVM boxing or host equality helpers.
3. Run companion coverage through `bash scripts/agent-stdlib-test`, add focused
   backend bit-inspection tests through `bash scripts/agent-test`, and verify
   normal standard-package loading, navigation, and generated documentation.

Implementation is complete only when every acceptance row has checked
evidence, and the source-surface, types, execution, and relevant standard-library
specifications explain the supported behavior. Add representative checked
usage examples at that point. Remove this proposal and its catalog entry once
the current specifications own the delivered contract. This documentation-only
proposal does not change current specifications or executable examples.
