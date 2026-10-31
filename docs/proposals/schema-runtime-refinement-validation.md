---
role: proposal
update-when: Schema validation predicate ordering, ordinary decoder composition, or their planned verification changes.
---

# Predicate Ordering and Decoder Composition for Runtime Validation

The implemented `validate MemberPath from Expr at Expr` operation and its
current source, typing, execution, and diagnostic contracts are specified in
[source surface](../specification/source-surface.md#schema-operations),
[types](../specification/types.md#inference-rules),
[execution](../specification/execution.md#schema-execution), and
[diagnostics](../specification/diagnostics-json.md#diagnostic-families).

This proposal retains only unimplemented predicate ordering and ordinary
decoder composition work. Recursive source-ADT payload traversal is current
behavior under the linked specification pages.

## Predicate Ordering

The existing expression, recursive traversal, and result shape would remain
unchanged:

```veln
validate ConnectedPacket from base_record at base_offset
```

After every refinement-bearing position succeeds, validation would evaluate
schema predicates. A composed child schema would run its predicates before the
containing field's `where` predicate. Field predicates would retain declaration
order, followed by the containing schema-level `validate` predicate. Predicate
failure would use `schema.validation_failed` at the field path or schema root.

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
| Predicates | Refinements precede predicates; child, field, and schema predicates follow the specified order. |
| External decoder | A real ordinary Veln byte decoder covers success, excluded variants, malformed tags and payloads, truncation, readiness, consumed counts, and unchanged bytes. |
| Failure preservation | Inputs and earlier successful values remain usable; no trap, partial refined value, or hidden decoder invocation occurs. |

Use checked semantic examples and a backend integration for predicate and
decoder representation and evaluation order. Any standard library helper added
for decoder composition requires companion coverage under
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
