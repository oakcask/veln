---
role: proposal
update-when: The proposed CancelOwner and observation-only CancelToken API, legacy token removal, or cancellation authority acceptance changes.
---

# Separate Cancellation Authority From Observation

Make cancellation authority a property of the argument type. This is a Ready
proposal independent of generic-function implementation. Existing nominal
opaque types can express the distinction.

## Current boundary

The [time API](../specification/effects.md#network-and-time-boundary-calls)
returns `CancelToken` both from `cancel_token()` and from
`cancel_token_from(owner)`. The former may be passed to `time::cancel`; the
latter fails there at runtime as observer-only. The
[JVM implementation](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-backend-jvm/src/runtime/effects.java.inc)
stores that difference in `observerOnly`. A well-typed call can therefore
request authority that the value was deliberately created not to possess.
This is a public capability issue, not a generic payload problem.

## Proposed API and authority

All functions retain `effects [time]`.

| Operation | Proposed signature and behavior |
| --- | --- |
| Create cancellation authority | `time::cancel_owner() -> CancelOwner` |
| Obtain observation capability | `time::cancel_token_from(owner: CancelOwner) -> CancelToken` |
| Request cancellation | `time::cancel(owner: CancelOwner) -> ()` |
| Observe cancellation | `time::is_cancelled(token: CancelToken) -> Bool` |

Every public `CancelToken` is observation-only. Wait and IO operations accept
tokens and cannot obtain an owner from them. There is no public constructor,
cast, projection, or implicit conversion from a token to an owner. An owner
is a shareable bearer capability, not a promise of linear or unique ownership.
Passing it to another task deliberately delegates cancellation authority.

Remove `cancel_token()`, `cancel_owned()`, and `is_cancelled_owner()`.
Existing owner cancellation migrates to `cancel(owner)`. Existing direct-token
code creates an owner and passes its projected token to consumers. Owner
status queries use `is_cancelled(cancel_token_from(owner))`; another status
family is unnecessary.

Cancellation is monotonic and idempotent. All tokens from the same owner
observe its cancellation; different owners remain independent. A successful
cancellation request does not wait for work or cleanup to finish. Existing
task cancellation remains a separate lifetime operation. This proposal does
not redesign the scoped Clock bridge's ability to simulate cancellation.

## Related work and alternatives

The inspected [Go context API](https://pkg.go.dev/context#WithCancel) returns
an observation `Context` and a separate `CancelFunc`. That supports separating
authority from observation. Veln does not adopt context key/value storage,
parent-child cancellation, Go interfaces, or callable cancellation handles.
It already has suitable nominal owner and token types.

Adding a second token subtype while retaining a mixed-authority `CancelToken`
would preserve the ambiguity. An always-fallible `cancel(token)` would make
permission failure explicit but still give every observer a meaningless
operation. Reusing `CancelOwner` states the precondition before execution.

## Planned acceptance and migration

| Case | Required observation | Intended verification |
| --- | --- | --- |
| Cancel with a token | Static argument-type rejection at the token; no execution. | Checker and human/JSON diagnostic coverage |
| Owner and its projected tokens | Initially active; all report cancellation after one request. | Stdlib time companions |
| Repeated and concurrent owner requests | Idempotent cancellation; no reset. | Stdlib companions and host race coverage where needed |
| Two owners | Cancelling one does not cancel the other. | Scoped Veln test support |
| Token used in existing wait/IO APIs | Observation still works; caller needs no cancellation authority. | Stdlib wait/network companions |
| Artifact and tooling boundaries | No legacy direct-token factory or token-taking cancel overload remains. | Signature, export, navigation, and symbol audit |

These are planned checks. Update source-less descriptors, opaque host values,
all clock/network/channel test support, examples, and consumers together.
Keep runtime resource validation as defense against malformed host values;
it is not a substitute for static authority separation.

No grammar changes are proposed. Formatting does not synthesize owners or
change call arguments; migration is explicit. Coordinate consumer migration
with the implemented
[cancellable wait outcome](../specification/effects.md#network-and-time-boundary-calls) and
[channel waits](channel-wait-api.md), but neither requires a different token
type for each wait operation. Use the effect-handler maintenance workflow.
Update the time boundary specification and executable evidence on landing,
then delete this page and its catalog entry.
