---
role: proposal
update-when: Explicit schema runtime validation syntax, erased input typing, refinement checks, decode adapter composition, or planned verification changes.
---

# Runtime Refinement Validation Through Schemas

Add an explicit runtime check that converts a base-typed immutable record into
the refined record described by a format-neutral schema. Ordinary decoder
functions can use this operation after decoding and projecting a complete base
value. This proposal defines a new checked operation; it is not current behavior.

## Source And Type Contract

```veln
schema ConnectedPacket
	state: State::Connected
end

# Proposed expression inside an ordinary decoder function:
validate ConnectedPacket from base_record at base_offset
```

The proposed expression grammar is
`validate MemberPath from Expr at Expr`. Schema lookup, public aliases,
private exact-companion access, and source spans follow the existing explicit
[schema operation rules](../specification/source-surface.md#schema-operations).
This expression does not change the schema-body `validate` predicate syntax.

| Input or result | Contract |
| --- | --- |
| Schema | A format-neutral schema with the currently eligible recursive visible shapes. Binary schemas are rejected by analysis. |
| Value | The schema-local record with each singleton or finite-set ADT refinement recursively replaced by its instantiated base ADT, including refinements inside generic arguments. Preserve nominal constructor identity, argument positions, visibility, and field requirements. |
| Offset | A `ByteOffset` supplied by the caller. The expression evaluates the value and offset once, in that order, before validation. |
| Success | `Ok(record)` in `Result<SchemaRecord, DecodeError>`, where `SchemaRecord` retains the schema's refinements. The value has the same contents and runtime representation as the input. |
| Failure | `Err(DecodeErrorWithReason(id, offset, field_path, reason))`. No refined result is published. The caller's input remains available and unchanged. |
| Existing decode and encode operations | Retain their current input types, result types, and capability checks. A base-typed value is accepted only by this new validation operation. |

The erased input type is derived from the selected schema, not from whichever
type the caller supplies. A wrong base ADT, generic argument, field shape, or
scalar type remains a static mismatch. Contextual record construction uses the
erased input expectation. This operation does not require general variance or
module immutable values.

For example, a schema position `Box<State::Connected>` accepts the base input
`Box<State>` and validates its actual payload. Erasure does not rewrite the
definition of `Box` or bypass its ordinary visibility rules.

Analysis rejects unresolved, ambiguous, wrong-kind, inaccessible, or binary
schema targets before execution using focused schema-operation diagnostics.
The primary span identifies the invalid target or operand; related notes carry
schema provenance, required types, and repair hints. Human and JSON coverage
must fix those diagnostics before implementation is complete.

## Validation Contract

First validate all refinement-bearing positions across the entire value.
Only when they all pass, evaluate schema predicates. Visit fields in declaration
order; at a composed-schema field, validate the child schema's predicates before
the containing field's `where` predicate. Evaluate each schema's own `validate`
predicate after its fields. This ordering allows a predicate to rely on the
schema's refined types. It does not change predicate ordering inside existing
decode operations.

| Position | Traversal and acceptance |
| --- | --- |
| Singleton refinement | The actual constructor must be the selected constructor. |
| Finite-set refinement | The actual constructor must belong to the selected set. Expected variants render in ADT declaration order. |
| Record | Visit fields in schema or declared payload field order. |
| Source ADT | Check a refinement at the current position before visiting the actual constructor's payloads in declaration order. Generic payloads use instantiated types. |
| `Option` and `Result` | Visit only the payload of the actual constructor. A constructor without a payload has no child to check. |
| `List` and `Vec` | Visit elements in increasing index order. |
| `Dict<String, T>` | Visit values by ascending lexicographic order of the keys' UTF-8 bytes, independent of insertion order or host locale. Keys are already statically `String`. |
| Composed format-neutral schema | Visit its fields at the containing position, preserving the containing path. |
| Finite value with recursive ADT type | Validate every actual finite payload occurrence. Repeated type identity alone must not suppress checks of later values. |

The supported vocabulary is the current
[format-neutral field vocabulary](../specification/source-surface.md#format-neutral-fields).
This proposal does not admit arbitrary dictionary-key types, opaque payloads,
or additional collection forms. Eligibility and public structural visibility
remain checked before execution.

The first failed refinement determines the error. Its id is
`schema.variant_refinement_mismatch`; its reason identifies the actual
constructor and the complete expected variant set. A failed target predicate
uses `schema.validation_failed`, with the failed predicate in the reason.
A negative supplied offset fails before traversal with
`schema.validation_offset_invalid` and a reason containing that offset.

Every error offset is exactly the supplied offset. This operation has no
per-node byte provenance and must not invent a constructor-tag offset. A decoder
that knows a narrower position can supply that offset itself.

The `field_path` is a JSON Pointer string. The first token is the terminal
schema declaration's name, independent of the alias used to select it;
subsequent tokens are record fields, collection indices, dictionary keys,
constructor names, and named payload fields or zero-based positional payload
indices along the failed position. Tokens escape `~` as `~0` and `/` as `~1`.
An error at the whole schema uses only its root token. Predicate failures use
the field's path or the schema root for a schema-level predicate.

## Decoder Composition And Wire Boundary

An ordinary source decoder owns its tag mapping, byte layout, and projection
from a binary schema-local record into a domain ADT. It returns its complete
base decode result before invoking validation. A wrapper follows this table:

| Base decoder result | Refined wrapper observation |
| --- | --- |
| `NeedMore(readiness)` | Preserve readiness; do not validate or publish a refined value. |
| `Invalid(error)` | Preserve the base failure, including malformed tags, malformed payloads, and truncation; do not validate. |
| `Decoded(base_record, consumed)` and validation succeeds | Return the refined record with the same consumed count. |
| `Decoded(base_record, consumed)` and validation fails | Return `Invalid(error)` through the existing failure channel. No partial refined value is published. |

A result-returning decoder uses the same success-only composition and returns
`Err` on validation failure. After operand evaluation, validation performs no
byte I/O and changes no wire bytes. It neither introduces a general ADT binary format nor weakens
`schema.variant_refinement_decode_unsupported` for generated binary decoders.
Merely observing a tag still does not strengthen a declared base result.

## Acceptance And Planned Verification

| Concern | Required evidence |
| --- | --- |
| Source and resolution | Accepted/rejected executable grammar cases, formatting round trips, schema aliases, imported public schemas, private companions, wrong-kind names, binary targets, and malformed operands. |
| Checked narrowing | Runtime and semantic cases for singleton, union, nested record, generic source-ADT payload, option, result, list, vector, dictionary value, and composed-schema positions; excluded constructors fail, admitted values retain their representation. |
| Static boundaries | Wrong base identity, wrong generic arguments, unsupported shapes, private constructors, and ordinary base-typed decode/encode operands remain rejected. |
| Ordering and paths | Multiple excluded positions, dictionary insertion permutations, escaped keys, positional/named payloads, and repeated recursive type occurrences produce the specified first path. |
| Predicate precedence | All refinement checks precede predicates; composed child predicates precede the containing field predicate, and field predicates precede their schema predicate. Alias selection preserves the terminal schema path. Error ids, paths, reasons, and supplied offsets are checked. |
| External decoder | A real ordinary Veln byte decoder with an explicit tag mapping exercises base success, excluded variants, malformed tags/payloads, truncation, readiness, consumed counts, and unchanged bytes. Hand-written constructor-only values do not replace this integration case. |
| Failure preservation | Input values and previously successful results remain usable after rejection; no trap, partial refined record, or hidden decoder invocation occurs. |
| Offset | Positive and zero offsets survive unchanged; a negative offset produces the defined recoverable error. |

All evidence is planned. Use checked source/runtime examples for the language
operation and backend contract. Any stdlib helper added during implementation
also requires its own companion tests under `crates/veln-stdlib/veln/`.
On implementation, update the source, type, execution, and diagnostic
specifications with executable evidence, then remove this proposal.

## Related Work And Alternatives

[Veln PR #1866](https://github.com/oakcask/veln/pull/1866), inspected at commit
`6a10e6502`, adds decoder capability rejection and explicitly leaves runtime
validation separate. The current binary shape classifier in
[`binary.rs`](../../crates/veln-sema/src/types/schema_types/binary.rs)
supports integer/byte layouts, schema composition, and dispatch, but supplies
no arbitrary source-ADT wire mapping. Therefore this proposal rejects the
assumption that generic ADT base decoding already exists.

The current [execution contract](../specification/execution.md#schema-execution)
makes record-to-domain projection ordinary source and already defines
schema predicate validation. The new operation adopts that separation and
uses a format-neutral schema as the validation shape. It adds reusable runtime
narrowing instead of a new `codec` declaration, `map to` clause, or implicit
cast. Fully hand-written matches remain possible but do not provide recursive,
schema-directed validation. Automatic binary ADT serialization is outside this
proposal and is unnecessary for its acceptance cases.
