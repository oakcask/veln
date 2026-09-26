---
role: specification
authority: normative
update-when: The veln-language skill contract, published language-reference contract, repository documentation authority, or checked scenario harness changes.
specification-coverage: usage=#usage; behavior=#routing-and-provenance; limits=#limits-and-failures
---

# Agent Language Routing

The `veln-language` skill gives agent clients one client-neutral route for Veln
language questions and repository tasks. Its canonical source is the
repository agent skill named `veln-language`. Client manifests, installation,
and adapters are outside this contract.

## Usage

Select the skill for a question about the Veln language or for a request to
inspect or change this repository. A language question requires the published
`search_docs` and `read_doc` tools. A repository task requires access to the
repository documentation tree.

## Routing and provenance

For a language question, the skill first calls `search_docs` with
`scope: "language"`. If a topic matches, it calls `read_doc` with the exact
snapshot topic URI from the first search result. This makes selection
deterministic when search returns multiple topics. The answer can contain only
claims from that resource and reports that exact URI as its source.

For repository inspection, changes, and proposal selection, the skill starts
at `docs/README.md`. It follows the smallest linked repository documentation
route for the request. A selected specification, proposal, or reference page
owns its subject. The proposal catalog is the terminal authority for proposal
availability and Ready-only selection. The published language reference is not
repository implementation authority. An explicit repository documentation
path takes precedence over a general subject phrase in the same request.

The requested action and subject determine the route by meaning. The skill
instructs clients not to decide from a closed vocabulary of verbs, question
words, or sentence frames. A request for the agent to examine, validate, or
alter Veln behavior or its implementation is repository work. This includes
synonymous actions such as assessing a parser. A request for information about
how a Veln language feature behaves remains a language question, including a
passive question such as how effects are handled.

A question whose subject is a compiler or parser implementation uses the
repository route. A request whose subject is the
repository, codebase, source code, or proposal state uses the repository route
without requiring an action verb. This includes direct and indirect questions
that put one of those explicit targets before or after the copula. Questions
about how Veln language features behave remain on the language route when they
request information rather than an inspection, test, or change action. Routing
treats an intent or explicit repository target as repository work only when it
is the requested action or subject. Merely mentioning one in a language
question, or asking what the term means, does not select the repository route.
A request to explain a stated language question also remains on the language
route.

## Limits and failures

The language route makes at most one search and one read. When search returns
no topic, the skill reports the absence and does not use proposal text or model
memory. The route distinguishes failures as follows:

| Tool result | Outcome |
| --- | --- |
| `search_docs` returns the transport error `tool_unavailable` | Stop after search and report that search is unavailable. |
| `read_doc` returns the transport error `transport_unavailable` | Stop after read and report that the selected topic is unavailable. |
| `read_doc` returns a tool error with code `resource_not_found` for the selected snapshot URI | Stop after read and report that the selected snapshot URI is stale. |

The skill selects these outcomes by the exact operation, result kind, and code
shown in the table. The stale-snapshot outcome additionally requires the error
URI to equal the URI selected from search. A different tuple does not select
one of these named failure outcomes.

Each failure reports the failed operation and the selected URI when one exists.
It also leaves any earlier successful result unchanged.
Within one server process, search candidates and reads remain retained state, so
a URI returned by `search_docs` remains readable. The stale-URI outcome can
occur after that server process ends and a replacement server starts with a
different checked language-reference snapshot. The replacement can reject the
earlier exact URI with `resource_not_found`.

Repository routing reads at most three distinct documentation files. It stops
when the selected route ends and reports when no route covers the request.
A documentation path contributes to a route only when it is the destination of
an actual Markdown navigation link. Link-shaped text in code, comments,
images, or escaped syntax does not make a path reachable.

## Verification

`workflow-scripts/fixtures/veln-language/scenarios.json` records the tool and
repository results for the acceptance model. The separate
`request-selection-oracle.json` file records the reviewed request text and its
expected action-and-subject classification. Run
`node workflow-scripts/check-veln-language-skill.mjs` to replay it against the
canonical skill. The workflow-script test suite checks the replay oracle,
the closed operative-contract and result shapes, exact failure dispatch,
provenance, bounded failures, preserved results, routing, and input limits. The
reviewed oracle independently
classifies the action and subject of each raw corpus request. The harness checks
that every replayed request matches that oracle, then applies the skill's closed
action-and-subject decision table. The corpus includes contrastive paraphrases,
synonymous requested actions, and incidental uses of action and repository
terms. It is finite evidence for the semantic instruction, not a general
natural-language classifier. Stress cases run in workers that the parent test
terminates at their time bound. The offline harness rejects repository
documents larger than `262144` bytes before parsing them, which bounds replay
resource use. It also verifies that a terminal authority recorded as current
does not declare a closed or superseded lifecycle. Stale search recordings are
checked in full against archived catalog evidence whose snapshot digest is
recalculated with the published catalog digest contract. The replay also
requires a distinct replacement server whose retained snapshot is the current
published snapshot before the stale read. The current published catalog bytes
must also match their digest sidecar before the harness parses them. The
harness bounds published
catalog bytes and topic count, archived snapshot count, overrides per snapshot,
and the snapshot-by-topic work product before it reconstructs archived
catalogs. Its scaling check measures accepted ASCII internal-space runs at
successively doubled sizes and allows bounded timing noise.
