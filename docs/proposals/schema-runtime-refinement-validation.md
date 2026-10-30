---
role: proposal
update-when: Recursive schema runtime validation, predicate ordering, decoder composition, or planned verification changes.
---

# Recursive Runtime Refinement Validation Through Schemas

Extend the implemented `validate MemberPath from Expr at Expr` operation beyond
direct fields, nested anonymous record fields, recursively composed
format-neutral schema fields, and the standard `Option`, `Result`, `List`,
`Vec`, and string-keyed `Dict` containers. Current behavior also traverses
acyclic same-module and visible imported source-ADT payload graphs. It validates
singleton and finite-set source-ADT refinements at those positions. Every
schema in the reachable composition graph must be format-neutral and
predicate-free, including branches without refinements. The current source,
typing, execution, and diagnostic contracts are specified in
[source surface](../specification/source-surface.md#schema-operations),
[types](../specification/types.md#inference-rules),
[execution](../specification/execution.md#schema-execution), and
[diagnostics](../specification/diagnostics-json.md#diagnostic-families).

This proposal retains only unimplemented recursive-value traversal, predicate
ordering, and ordinary decoder composition work. Acyclic source-ADT payload
traversal is current behavior under the linked specification pages.

## Proposed Extension

The existing expression, composed-schema traversal, and result shape remain
unchanged:

```veln
validate ConnectedPacket from base_record at base_offset
```

The value input remains the schema-local record with every refinement-bearing
position replaced by its base ADT. The extension would derive that erased type
through recursive values in addition to the implemented record, composition,
standard-container, and acyclic source-ADT payload traversal. It would preserve
nominal identity, generic argument positions, visibility, and field
requirements. Unsupported shapes and binary schemas would remain static
failures.

Runtime traversal would cover the same recursive positions. It would retain
the current representation-preserving `Result<SchemaRecord, DecodeError>`
success, exact supplied offsets, terminal-schema JSON Pointer roots, and
recoverable `schema.variant_refinement_mismatch` and
`schema.validation_offset_invalid` failures.

## Traversal And Predicate Ordering

| Position | Proposed traversal |
| --- | --- |
| Recursive finite value | Validate every actual payload occurrence; repeated type identity does not suppress later values. |

After every refinement-bearing position succeeds, validation would evaluate
schema predicates. A composed child schema would run its predicates before the
containing field's `where` predicate. Field predicates would retain declaration
order, followed by the containing schema-level `validate` predicate. Predicate
failure would use `schema.validation_failed` at the field path or schema root.

Recursive-value paths would reuse the implemented schema, composed field,
collection-index, dictionary-key, source-ADT constructor, and named or
positional payload tokens. Tokens would keep the implemented `~0` and `~1`
escaping.

## Ordinary Decoder Composition

An ordinary source decoder owns its tag mapping, wire layout, and projection
to a complete base value. A proposed wrapper would validate only a successful
base result:

| Base decoder result | Refined wrapper observation |
| --- | --- |
| `NeedMore(readiness)` | Preserve readiness and do not validate. |
| `Invalid(error)` | Preserve the base failure and do not validate. |
| `Decoded(base, consumed)` with successful validation | Return the same value and consumed count at the refined type. |
| `Decoded(base, consumed)` with failed validation | Return `Invalid(error)` without publishing a partial refined value. |

A result-returning decoder would follow the same success-only composition and
return `Err` on validation failure. Validation would perform no byte I/O,
change no wire bytes, and would not weaken
`schema.variant_refinement_decode_unsupported` for generated binary decoders.

## Acceptance And Planned Verification

| Concern | Required evidence |
| --- | --- |
| Recursive typing | Recursive-value erasure through already supported source-ADT payloads; wrong identities and generic arguments remain rejected. |
| Runtime traversal | Admitted and excluded constructors at every recursive position, including repeated recursive values. |
| Ordering and paths | Recursive-value occurrence order using the implemented source-ADT constructor and payload path segments. |
| Predicates | Refinements precede predicates; child, field, and schema predicates follow the specified order. |
| External decoder | A real ordinary Veln byte decoder covers success, excluded variants, malformed tags and payloads, truncation, readiness, consumed counts, and unchanged bytes. |
| Failure preservation | Inputs and earlier successful values remain usable; no trap, partial refined value, or hidden decoder invocation occurs. |

Use checked semantic and runtime examples for recursive-value traversal and a
backend integration for representation and evaluation order. Any standard
library helper added for decoder composition requires companion coverage under
`crates/veln-stdlib/veln/`.

## Related Boundary

[Veln PR #1866](https://github.com/oakcask/veln/pull/1866) introduced generated
decoder capability rejection and kept runtime validation after base decoding as
separate work. The current binary shape classifier in
[`binary.rs`](../../crates/veln-sema/src/types/schema_types/binary.rs) supports
integer and byte layouts, schema composition, and dispatch, but supplies no
generic source-ADT wire mapping.
This proposal therefore keeps decoding and runtime narrowing separate. It does
not introduce automatic ADT serialization, a new `codec` declaration, a
`map to` clause, or an implicit cast.
