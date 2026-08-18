---
role: proposal
update-when: The MCP JSONL assertion contract, executable definition-fixture response set, shared-capture evidence boundary, or saved-reference slice status changes.
---

# Agent Language Services Slice Closure

## Summary

Establish the executable-evidence boundary for agent-language-service slices
before retrying saved workspace function references. Add response-local MCP
JSONL assertions and state when shared invariant evidence can be composed with
adapter evidence.

The saved-reference adapter is not implemented by this proposal. A later
target may reintroduce that bounded slice only after this proposal's closure
gate passes.

## Selection State

This proposal is ready. Implement the JSONL assertion surface and convert the
definition fixture as one bounded evidence change. The saved-reference adapter
remains separate work.

## Problem

MCP stdio cases also need to assert results that contain workspace-specific
canonical `file:` URIs. Exact stream fixtures cannot name the temporary
workspace root. Global `contains` checks avoid that dynamic value, but they do
not bind locations, ordering, or counts to one JSON-RPC response. A correct
runtime result can therefore coexist with an executable case that does not
prove the claimed contract.

The stable-capture invariant is shared across saved navigation adapters. The
current acceptance wording does not say whether every adapter must reproduce a
source mutation during capture or whether a shared invariant test plus an
adapter failure-mapping test is sufficient. That ambiguity encourages
adapter-specific tests that exercise a different failure trigger while being
described as source-mutation evidence.

## Goals

- Let executable MCP cases select one JSONL response and assert nested values,
  array order and length, missing values, and canonical workspace `file:` URIs.
- Define a compositional evidence rule for shared capture invariants and
  adapter-visible failure mapping.
- Route the implemented JSONL assertion contract to
  `docs/reference/toolchain-test-harness.md` and its checked harness evidence.

## Non-Goals

- Implementing `references`, expanding the shared navigation symbol set, or
  changing Veln name resolution.
- Adding dependency or standard-library reference search, pagination, cursors,
  retained resources, documentation tools, or client plugins.
- Requiring every adapter to duplicate the same filesystem race harness.
- Treating implementation records or active proposals as current behavior.
- Adding a general JSON query language to the toolchain case format.

## MCP JSONL Assertion Contract

Add a small toolchain-case assertion surface for newline-delimited JSON output.
The exact manifest spelling is an implementation choice, but the checked
contract must support these observations:

| Observation | Required assertion behavior |
| --- | --- |
| Select a response | Select exactly one JSON object by a string or integer JSON-RPC `id`. An assertion fails when that ID has no match or more than one match. Other response IDs remain valid input. |
| Select a value | Apply an RFC 6901 JSON Pointer to the selected response. An invalid pointer or missing intermediate value fails unless absence is the expected operation. |
| Compare a value | Compare a complete JSON value for equality. Object member order is ignored. Arrays preserve order and length. Strings, booleans, null, and integers compare by decoded value; other numbers compare by their preserved JSON spelling. |
| Check an array length | Assert the exact length of the array at one pointer. A non-array value fails. |
| Check absence | Assert that one pointer does not resolve. |
| Check a canonical workspace location | Compare a string with the canonical `file:` URI derived from one existing regular case-workspace-relative file. The expectation rejects absolute paths, empty segments, `.`, `..`, backslashes, and link-like traversal. URI construction uses the same workspace-file URI contract as MCP. |

The stream decoder must decode every nonempty line as one JSON object. It must
reject malformed JSON and non-object lines. Each assertion then requires
exactly one response with its selected ID. Harness unit tests must cover string
and integer IDs, missing and duplicate selected IDs, unrelated IDs, escaped
JSON Pointer segments, reordered object members, ordered arrays, array length,
missing values, dynamic workspace URIs, and every rejection row above. The
ordered-array test uses at least two distinguishable values and proves that the
reversed expectation fails.

Stream-wide `contains` and `not_contains` remain available for incidental text
or discovery checks. They are not sufficient evidence for response-local MCP
locations, order, cardinality, or failure payloads.

## Executable Definition Evidence

Convert the `definition-workspace` case without reducing its coordinate and
failure coverage. Response-local assertions must observe every definition call
in its checked input:

| Response | Required observation |
| --- | --- |
| ID 3 | Canonical workspace URI, exact range, singleton content cardinality and indexed content type, `isError: false`, and absent protocol error. |
| ID 4 | Successful no-definition result and absent protocol error. |
| ID 5 | `invalid_position`, `isError: true`, and absent success-only definition. |
| IDs 6 and 7 | Integral decimal and exponent coordinates return the same canonical URI and exact range as ID 3. |
| IDs 8 and 10 | Oversized positive coordinates return `invalid_position` with `isError: true`. |
| IDs 9 and 11 | Non-integer coordinates return protocol invalid params and have no result. |

The singleton content assertion proves the definition result's actual
cardinality and index. It does not claim to prove a multi-location ordering
contract. Generic JSON array order belongs to the two-or-more-element harness
unit test because this executable definition result has only one location.
Raw stream assertions may remain for initialization and tool discovery text.
They must not replace any response observation in the table.

## Shared Capture Evidence

A saved-navigation adapter may compose evidence across ownership boundaries
when all rows in this table are present:

| Claim | Required evidence owner |
| --- | --- |
| Source identity or bytes changing during capture cannot produce a successful stable snapshot. | The shared capture boundary has a deterministic mutation or identity-change test. |
| The adapter calls that shared boundary for the operation under review. | A focused adapter test or a narrow structural test identifies the route without replacing it with a weaker capture path. |
| Exhausted capture returns `snapshot_changed`. | The adapter-level tool result test asserts the domain code and `isError: true`. |
| Failure publishes no partial operation payload. | The same adapter-level result asserts that success-only fields are absent. |

An adapter does not need another timing-sensitive source mutation test when
these four rows are checked. If the adapter buffers or transforms partial
results before capture succeeds, it needs its own deterministic failure seam
and atomic-publication test.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Preserve generic JSON array order. | Equality accepts the expected two-or-more-element order and rejects the reversed order; exact length remains independently assertable. | Focused toolchain-harness unit cases. |
| Convert the executable definition case. | IDs 3 through 11 have every response-local observation in the executable definition evidence table, while raw checks remain only for incidental initialization and discovery text. | One MCP executable fixture using the new assertion surface and the checked semantic baseline. |
| Select malformed, missing, or duplicate JSONL responses. | The harness rejects each case with an actionable assertion failure. | Table-driven harness rejection tests. |
| Reuse the shared capture boundary from an adapter. | Shared mutation evidence and adapter mapping evidence jointly prove `snapshot_changed` with no partial success fields. | Shared capture test plus focused adapter result test, recorded together in the slice evidence map. |
| Publish the implemented harness contract. | The normative harness reference defines JSONL selection, equality, length, absence, and workspace-file URI assertions and points to the checked unit and semantic-baseline evidence. | Update `docs/reference/toolchain-test-harness.md`, harness tests, one executable case, and the checked semantic baseline. |

## Completion Rule

This proposal completes when each acceptance row passes and the implemented
assertion behavior is published in the normative harness reference with links
to its checked unit and executable-case evidence. Move the completed record
out of `docs/proposals/` before selecting the saved-reference target.
