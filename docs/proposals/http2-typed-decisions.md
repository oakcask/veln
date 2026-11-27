---
role: proposal
update-when: The proposed HTTP/2 typed decision projections, public payload preservation, branch-string retirement, or conversion-family readiness changes.
---

# Typed HTTP/2 Decision Projections

Expose typed decision views so consumers can branch once and receive the
corresponding payload. This is a Design In Progress proposal until each
in-scope decision family has a complete projection map. It does not require
generic functions: existing ADTs and records can express family-specific views.

## Current boundary

The exported [HTTP/2 core facade](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/http2/core.veln)
returns opaque decision types and publishes `*_status() -> String` plus
separate state/failure getters. For example,
`stream_lifecycle_transition_status`, `stream_lifecycle_transition_state`,
and `stream_lifecycle_transition_failure` require string discrimination and
then a second fallible extraction. The internal
[transition decision](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/http2/core/stream_transition.veln)
already contains either a state or a typed failure.

[Connection driving code](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/http2/connection.veln)
uses similar status/getter sequences for local settings and frame decisions.
A misspelled discriminator becomes a fallback branch, and the checker cannot
establish that a status test makes a later getter succeed. Generic functions
could reduce implementation repetition but are not needed for typed outcomes.
There is no inspected historical evidence that their absence caused this API.

## Proposed projection model

Keep invariant-bearing internal state and decision constructors opaque.
Provide a pure typed projection of each public decision instead of making
private constructors universally public. Projection values are observations;
constructing one does not grant authority to manufacture an internal decision.

For the two-case lifecycle transition, the planned projection is:

```veln
http2::core::stream_lifecycle_transition_result(decision: CoreStreamLifecycleTransitionDecision) -> Result<CoreConnectionState, CoreStreamLifecycleTransitionFailure>
```

For families with more cases, return a family-specific public view ADT.
Each case carries all payload available from its corresponding decision,
including bytes, state, stream identifiers, and encode-failure details.
For example, `CoreDataSendDecision` requires `Accepted`, `Rejected`, and
`EncodeFailed` views; it must not merge the latter two or drop a committed
state or output. A response with `NoResponse` keeps an explicit case.
Use the [ADT naming policy](../reference/standard-library-adt-naming.md).

Retire status strings and incompatible-case getters once their family has a
complete view and all consumers migrate. Retain text-rendering helpers that
serve diagnostics, stable protocol identifiers, or display. A string return
type alone is not evidence that an API should be removed.

## Scope and readiness

The planned first delivery covers externally exposed operation-decision
families reached through `http2::core`: lifecycle/stream admission, receive,
flow control, shutdown, outbound data/control/headers, ping, and settings.
Failure diagnostics and HPACK error-kind renderers are outside this delivery.
Connection service consumers must use the new projections where they consume
these decisions; a separate redesign of service failure ADTs is not required.

Before Ready promotion, map every affected public decision to its complete
public view, signatures, field meanings, and retired getters. In particular,
encode failures can carry state and partial output that must not disappear.
The existing [outbound data decision](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/http2/core/outbound_data_flow.veln)
and [ping decisions](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/http2/core/ping.veln)
show why a universal accepted/rejected Boolean cannot express every family.
The projection map is the remaining design deliverable, not a demand for a
prototype or passing implementation tests before Ready. Each row must name
the current decision, every replacement case and payload, projection signature,
field meaning, and retired status/getter path. Unmapped families remain a
design blocker; consumer migration and the acceptance tests below are
implementation completion work. Generic-function implementation is not a
prerequisite for choosing these family-specific views.

## Related work and alternatives

The inspected [Rust Result definition](https://doc.rust-lang.org/std/result/enum.Result.html)
contains a success payload or a failure payload. It supports the two-case
projection above. Three-case Veln decisions need a dedicated ADT rather than
nested strings or collapsing encode failure into rejection. Veln's private
constructors and protocol state invariants limit a direct public-enum rewrite.

Publishing a status enum alone would remove spelling errors but leave the
second fallible extraction. Exposing every private constructor could bypass
validation. Typed projections preserve the boundary while improving branching.
Generic projection helpers may follow generic-function support, but cannot
replace an inventory of each decision's material payload.

## Planned acceptance

| Case | Required observation | Intended verification |
| --- | --- | --- |
| Accepted/rejected lifecycle | One match yields exactly state or typed failure without another extraction error. | Stdlib HTTP/2 companions |
| Accepted/rejected/encode-failed output | View retains case, bytes, state, and all case-specific details. | Companion tables using independent expected payloads |
| Ping acknowledgement/no response | Explicit cases; no empty byte sentinel for an absent response. | Stdlib companions |
| Public construction of a view | Cannot be supplied where an opaque decision is required. | Checker rejection case |
| Driver migration | Same protocol bytes, state transitions, diagnostics, and failure precedence. | Existing connection companions plus focused backend contract cases |
| Removed branch helpers | No consumer branches on retired status strings or requires impossible-case getters. | Symbol audit and API documentation/navigation checks |

No row is currently implemented. Source syntax is unchanged. Formatter output
uses existing calls and matches; migration is explicit because a formatter
cannot infer the replacement case payloads. Current authority remains the
[HTTP/2 specification](../specification/http2.md). Update it with executable
evidence when a complete delivery lands, keep only incomplete scope here,
and remove this proposal and its catalog entry when finished.
