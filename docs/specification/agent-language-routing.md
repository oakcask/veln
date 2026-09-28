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
`scope: "language"`. The agent derives one broad English topic term that names
the main Veln language concept in the request. It translates a non-English
request. It uses multiple words only for an established compound concept such
as `borrow checker`. It omits `Veln`, question framing, requested answer form,
operations or details being asked about, and incidental concepts. The complete
request is not a query unless the request already consists only of the topic
term. This prevents question grammar and narrow details from becoming
mandatory search tokens.

The derived query contains at most 64 Unicode scalar values. If the agent
cannot identify one main language concept, or if the derived query is empty or
too long, it stops without a tool call and reports that no bounded query can be
derived. If a topic matches, the skill calls `read_doc` with the exact snapshot
topic URI from the first search result. This makes selection deterministic
when search returns multiple topics. The answer can contain only claims from
that resource and reports that exact URI as its source.

For repository inspection, changes, and proposal selection, the skill starts
at `docs/README.md`. It follows the smallest linked repository documentation
route for the request. A selected specification, proposal, or reference page
owns its subject. The proposal catalog is the terminal authority for proposal
availability and Ready-only selection. The published language reference is not
repository implementation authority. An explicit repository documentation
path takes precedence over a general subject phrase in the same request. For
such a path, the skill reads `docs/README.md` first and then reads the named
path directly. The path must be a normalized existing Markdown path under
`docs/` and a current documentation authority. It does not need to be linked
directly from `docs/README.md`.

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
| `search_docs` returns any other failed or malformed result | Stop after search and report that published language-reference search failed. |
| `read_doc` returns any other failed or malformed result | Stop after read and report that the selected topic could not be read. |

The skill evaluates the rows in table order. It selects the first three named
outcomes by the exact operation, result kind, and code shown in the table. The
stale-snapshot outcome additionally requires the error URI to equal the URI
selected from search. Every other failed or malformed search or read selects
the matching generic bounded outcome instead of escaping failure handling.

Each failure reports the failed operation and the selected URI when one exists.
It also leaves any earlier successful result unchanged.
Within one server process, search candidates and reads remain retained state, so
a URI returned by `search_docs` remains readable. The stale-URI outcome can
occur after that server process ends and a replacement server starts with a
different checked language-reference snapshot. The replacement can reject the
earlier exact URI with `resource_not_found`.

Linked repository routing reads at most three distinct documentation files.
An explicit documentation path uses exactly two reads: `docs/README.md` and
the named terminal authority. Routing stops when the selected route ends and
reports when no route covers the request.
Outside the explicit-path rule, a documentation path contributes to a route
only when it is the destination of an actual inline or reference-style
Markdown navigation link. An inline destination takes precedence when a label
also has a reference definition. Link-shaped text in code, comments, images,
or escaped syntax does not make a path reachable.

## Verification

`workflow-scripts/fixtures/veln-language/scenarios.json` records the tool and
repository results for the acceptance model. The separate
`request-selection-oracle.json` file records each reviewed raw request and its
expected observable route. `language-query-oracle.json` independently records
the accepted normalized queries for language scenarios. Run
`node workflow-scripts/check-veln-language-skill.mjs` to replay it against the
canonical skill. The workflow-script test suite checks the replay oracle,
the closed operative-contract and result shapes, exact failure dispatch,
provenance, bounded failures, preserved results, routing, and input limits. The
reviewed routing oracle is independent of the replay's action-and-subject
labels. The harness checks that every replayed raw request matches the oracle
route and that the replay labels select that route through the skill's closed
decision table. The query oracle checks representative English, non-English,
absent-topic, and non-derivable requests. Both corpora are
reviewed finite evidence for semantic instructions, not executable general
natural-language classifiers.

The harness rejects oversized or excessive-work skill, schema, documentation,
catalog, snapshot, and fixture inputs with a bounded failure. Stress checks use
a time bound and terminate nonresponsive work before reporting failure. These
limits keep offline replay bounded without making its parsing, validation, or
fixture-construction algorithms part of the routing contract.

File-backed inputs must be regular files before the harness consumes them. A
non-regular scenario input fails without waiting for a producer. When the
harness opens a repository route document, the opened file must remain inside
`docs/`; a concurrent link substitution outside that boundary fails the check.

The harness also checks that a selected repository authority is current.
Recorded search results must match checked published or archived catalog
evidence and its snapshot digest. A stale snapshot scenario must replace the
server that returned the URI with a distinct server that retains the current
published snapshot before the read fails.
