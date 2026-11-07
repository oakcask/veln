---
role: reference
authority: normative
update-when: Standard-library ADT naming conventions, constructor qualification support, or the library examples used to explain those conventions change.
---

# Standard-Library ADT Naming

This policy governs names chosen for public and private standard-library ADTs
and their variants. Apply it when adding or revising an ADT. The goal is to
make each case understandable in its owning type without repeating the same
context in every constructor.

The language's identifier casing and name-resolution rules remain defined in
[name-resolution.md](../specification/name-resolution.md). This policy guides
library design; it does not add compiler restrictions on user-defined names.

## Type Names Carry Shared Context

Name an ADT for the value, state, decision, or failure it represents. Include
the domain or operation needed to distinguish that type at its public API
boundary. Suffixes such as `Outcome`, `Decision`, `Lifecycle`, and `Failure`
are useful when they explain that role; they are not a required vocabulary.

Keep distinct concepts in distinct types even when they have similar cases.
`StreamWriteOutcome` and `CancellableWaitOutcome`, for example, can both have
`Completed`, `DeadlineExpired`, and `Cancelled` variants without becoming one
shared outcome type.

## Variant Names Identify Cases

Within a type, choose names for the differences a caller must handle. Remove
words that only repeat the owning type's domain, operation, or result category.
Do not add a common prefix solely to make constructors unique across ADTs.

| Owning ADT | Variant examples | Naming basis |
| --- | --- | --- |
| `CoreDataSendDecision` | `Accepted`, `Rejected`, `EncodeFailed` | The type identifies the operation; the variants identify its outcomes. |
| `StreamReadOutcome` | `Chunk`, `End`, `DeadlineExpired`, `Cancelled` | The type identifies a stream read; the cases distinguish data, clean end, deadline, and cancellation. |
| `CoreStreamLifecycle` | `Open`, `HalfClosedLocal`, `HalfClosedRemote`, `Reset` | Each name identifies a state; local and remote distinguish different states. |
| `DecodeError` | `WithoutReason`, `WithReason` | The names distinguish whether an explanatory reason is carried. |
| `DiagnosticDestination` | `StandardError`, `File`, `Disabled` | The type identifies what is routed; the variants identify where or whether it is written. |

Shared words are appropriate when they explain case-specific behavior or a
meaningful group of cases. `DecodeReadiness::NeedBytes` and
`DecodeReadiness::NeedEnd` retain `Need` because both describe a requirement on
future input. `HalfClosedLocal` and `HalfClosedRemote` retain `HalfClosed`
because it distinguishes those states from `Open` and `Closed`. A common
substring alone is not a naming violation.

A type with one constructor can use the type name for that constructor, as
`Byte`, `ByteCount`, and `ByteOffset` do. There are no sibling cases to
distinguish, and replacing these names with a generic constructor word would
hide the value's meaning. If the type gains more cases, review the names as a
set.

## Qualify Overlapping Names

Use the owning type to disambiguate overlapping constructor leaves, for
example `StreamInput::Chunk(bytes)` and `StreamReadOutcome::Chunk(bytes)`.
Add the module or package qualifier when the type itself needs disambiguation.
Use this form in examples that compare different ADTs with the same leaves.

A bare constructor is appropriate when lookup or the available type context
selects its owner unambiguously. Do not assume that an unannotated binding or
a generic call argument supplies that context. Resolve ambiguity with a type
annotation or a qualified constructor path rather than lengthening every
variant name in the type.

## Review and Verification

Authors and reviewers evaluate the type and its cases together. For a repeated
word, identify the distinction it conveys beyond the owning type. Check a
construction and a match at a representative call site, including overlapping
leaves when relevant. A name that loses a required distinction when shortened
keeps that distinction; a name that only repeats the owner loses the
redundancy.

This is a semantic review rule. There is no automatic prefix prohibition:
text matching cannot determine whether a shared word conveys a requirement,
a state family, or redundant context. Existing compiler and companion tests
verify resolution and API behavior, not the quality of a name.

When renaming variants, update constructor calls, patterns, refinements, host
descriptors and runtime tags, rendered and JSON value expectations, current
specifications, and generated documentation wherever they depend on the
name. Follow the [test placement policy](toolchain-test-harness.md#test-placement-policy)
for behavioral coverage. Reconsider a name when the ADT gains cases, its API
boundary changes, or available qualification no longer makes the cases clear.

## References

- Outcome and decoder declarations:
  [`prelude.veln`](../../crates/veln-stdlib/veln/prelude.veln).
- Send decisions:
  [`outbound_data_flow.veln`](../../crates/veln-stdlib/veln/http2/core/outbound_data_flow.veln).
- Stream states:
  [`stream_state.veln`](../../crates/veln-stdlib/veln/http2/core/stream_state.veln).
- Diagnostic destinations:
  [`observe.veln`](../../crates/veln-stdlib/veln/observe.veln).
- Single-constructor byte values:
  [`bytes.veln`](../../crates/veln-stdlib/veln/bytes.veln).
