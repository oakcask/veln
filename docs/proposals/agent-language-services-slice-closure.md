---
role: proposal
update-when: The agent-language-services proposal lifecycle, MCP JSONL assertion contract, shared-capture evidence boundary, or saved-reference slice status changes.
---

# Agent Language Services Slice Closure

## Summary

Establish one finite completion boundary for agent-language-service slices
before retrying saved workspace function references. Separate implemented
history from the active proposal, add response-local MCP JSONL assertions, and
state when shared invariant evidence can be composed with adapter evidence.

The saved-reference adapter is not implemented by this proposal. A later
target may reintroduce that bounded slice only after this proposal's closure
gate passes.

## Problem

The active agent-language-services proposal currently serves two roles. It is
both the plan for unresolved capabilities and a ledger of implemented slices.
Each completed slice therefore adds more current-status prose and implemented
acceptance rows to an active proposal. Removing only the newest target does not
leave the proposal with only unimplemented work.

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

- Keep `docs/proposals/agent-language-services.md` limited to unresolved
  requirements, dependencies, non-goals, and a finite completion gate.
- Preserve useful implemented history in one supporting implementation record
  without making that record authority for current behavior.
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

## Lifecycle Boundary

The agent-language-services proposal must be split by lifecycle, not by copying
the complete page into two active documents.

| Content class | Required destination |
| --- | --- |
| Current observable behavior | The matching page under `docs/specification/` and checked examples. |
| Completed slice rationale and evidence routes that remain useful | One supporting record under `docs/reference/implemented-proposals/`. |
| Unresolved externally observable requirements and acceptance rows | `docs/proposals/agent-language-services.md`. |
| Rejected, superseded, or obsolete plan text | Remove it, preserving rationale under `docs/reference/` only when it remains useful. |

The active proposal may link to a current specification to state a prerequisite
or an exclusion. It must not retain an implementation-status inventory,
implemented acceptance rows, or a chronological slice ledger. The supporting
record must state that current behavior is defined by specification and
executable evidence, not by the record.

After the split, every acceptance row in the active proposal names planned
evidence. The existing closed capability matrices and enumerated completion
gate remain the stopping condition. Adding a new capability requires a new
proposal or an explicit revision of that finite gate; completing a slice does
not append another implemented row to the active proposal.

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
missing values, dynamic workspace URIs, and every rejection row above.

Stream-wide `contains` and `not_contains` remain available for incidental text
or discovery checks. They are not sufficient evidence for response-local MCP
locations, order, cardinality, or failure payloads.

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
| Split the umbrella proposal. | The active page contains only unresolved work; useful completed history is in one supporting implementation record; current behavior routes to specification and executable examples. | Documentation lifecycle review, frontmatter validation, link validation, and a stale implemented-row search. |
| Assert an ordered MCP result with a dynamic workspace URI. | One response ID has the expected array length; each index has the expected range and a URI equal to the canonical URI for its case-relative file. Together these indexed assertions prove order, cardinality, and complete location content without literal dynamic-URI array equality. | Toolchain-harness unit case and one MCP executable fixture using the new assertion surface. |
| Select malformed, missing, or duplicate JSONL responses. | The harness rejects each case with an actionable assertion failure. | Table-driven harness rejection tests. |
| Reuse the shared capture boundary from an adapter. | Shared mutation evidence and adapter mapping evidence jointly prove `snapshot_changed` with no partial success fields. | Shared capture test plus focused adapter result test, recorded together in the slice evidence map. |
| Publish the implemented harness contract. | The normative harness reference defines JSONL selection, equality, length, absence, and workspace-file URI assertions and points to the checked unit and semantic-baseline evidence. | Update `docs/reference/toolchain-test-harness.md`, harness tests, one executable case, and the checked semantic baseline. |
| Complete this proposal. | Harness support, lifecycle split, capture-evidence rule, normative harness reference, and checked evidence are complete; no requirement in this page remains planned. | Final proposal audit, then move this page to implemented proposal records and remove its catalog entry. |

## Implementation Order

1. Add and test the MCP JSONL assertion surface.
2. Move implemented agent-language-service history to a supporting record and
   reduce the active proposal to its unimplemented remainder.
3. Document the shared-capture evidence composition in the active proposal's
   acceptance map.
4. Update `docs/reference/toolchain-test-harness.md`, one executable JSONL
   case, and the semantic baseline with the implemented assertion contract.
5. Complete the proposal lifecycle audit. A later proposal selection may then
   issue a new bounded saved-reference target.
