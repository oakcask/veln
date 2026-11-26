---
role: proposal
update-when: The proposed removal of fixed schema sample decoders from the public prelude, compiler descriptors, or runtime mappings changes.
---

# Retire Sample-specific Prelude APIs

Remove the fixed-schema sample decoders from the public standard library.
This is a Ready proposal for removal, not a claim that the symbols are absent.
It does not depend on generic functions.

## Current boundary and rationale

The exported [prelude](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/prelude.veln) publishes
`byte_decode_schema_width_sample` and `byte_decode_schema_validation_sample`.
The [JVM collections runtime](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-backend-jvm/src/runtime/collections.java.inc)
implements them using fixed `SchemaWidthSample` and `SchemaValidationSample`
layouts. Their names and returned records identify examples, rather than a
reusable consumer capability. The compiler also recognizes their bare names
in [standard symbols](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-sema/src/standard_symbols.rs).
Removing only the source wrappers would leave another public route.

The historical absence of generic functions does not justify retaining these
fixed schema entry points. No inspected record establishes why they were
originally exported. This proposal makes no claim about that historical intent.

## Proposed contract

| Surface | Planned result |
| --- | --- |
| Bare and prelude-qualified sample calls | Name lookup rejects both removed functions. |
| Explicit standard-package imports and generated docs | Neither function is exported or offered. |
| Compiler descriptors, completion, and JVM mappings | No source-callable sample path remains. |
| General binary schemas and primitive byte readers | Existing decoding and validation behavior is preserved. |

Backend tests that need these two layouts must declare them in test inputs
and exercise the general schema codec. If a backend-only fixture factory is
still useful, keep it in test harness code without a source-callable intrinsic.
Do not move a special-purpose sample API into a newly exported test module.
Keep `byte_expect_fixed_u8_be` outside this removal: its parameterized contract
needs a separate justification rather than guilt by proximity.

## Related work and alternatives

The inspected [Rust std prelude documentation](https://doc.rust-lang.org/std/prelude/index.html)
describes a small set of widely useful automatically imported items. Veln's
lookup mechanism differs, so Rust's exact item list is not adopted. The design
consequence is narrower: an example-specific layout has no prelude role.
Veln's inspected fixed-schema runtime implementations establish that these
particular functions are samples; their names alone are not the evidence.
Keeping aliases would preserve the same unnecessary public contract, so the
experimental project should remove them.

## Migration and planned acceptance

| Case | Required observation | Intended verification |
| --- | --- | --- |
| Bare, qualified, and function-value lookup | All removed spellings are unavailable; unrelated byte helpers still resolve. | Checker tests and generated API/navigation checks |
| Width sample | The general schema path decodes the independently specified two- and four-byte fields. | Backend codec fixture |
| Validation sample | The general path accepts valid lengths and rejects padding longer than the declared length. | Schema behavior fixture |
| Public inventory | No sample descriptor, export, completion, or documentation entry survives. | Export checks and symbol-removal audit |

Follow the [test placement policy](../reference/toolchain-test-harness.md#test-placement-policy):
stdlib byte API assertions stay in companion tests; backend fixtures establish
general schema emission or codec integration, not replacement stdlib coverage.
No planned row is currently reported as passing.

Use the symbol-rename audit workflow to classify residual sample names in
fixtures and historical material. There is no syntax extension. The formatter
keeps ordinary calls unchanged and does not rewrite removed names; migration
is an explicit edit to fixture declarations and calls. Current authority is
[prelude helpers](../specification/prelude-helpers.md) and
[execution](../specification/execution.md). Update those pages and executable
evidence when removal lands, then delete this proposal and its catalog entry.
