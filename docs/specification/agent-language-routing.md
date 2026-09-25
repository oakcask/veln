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

An inspection or change request uses the repository route even when the intent
verb does not start the request. A passive question that asks how a compiler or
parser implementation is implemented also uses the repository route. A request
whose subject is the repository, codebase, source code, or proposal state uses
the repository route without requiring an action verb. Questions about how
Veln language features behave remain on the language route whether the Veln
subject appears before or after the feature named in the question. Routing
treats an intent or explicit repository target as repository work only when it
is the requested action or subject. Merely mentioning one in a language
question does not select the repository route.

## Limits and failures

The language route makes at most one search and one read. When search returns
no topic, the skill reports the absence and does not use proposal text or model
memory. When search is unavailable, a selected topic cannot be read, or a
snapshot URI is stale, the skill stops, reports the failed operation and
selected URI when one exists, and leaves any earlier successful result
unchanged.

Repository routing reads at most three distinct documentation files. It stops
when the selected route ends and reports when no route covers the request.

## Verification

`workflow-scripts/fixtures/veln-language/scenarios.json` records the tool and
repository results for the acceptance model. Run
`node workflow-scripts/check-veln-language-skill.mjs` to replay it against the
canonical skill. The workflow-script test suite checks the replay oracle,
closed result shapes, provenance, bounded failures, preserved results, routing,
and input limits. Stress cases run in workers that the parent test terminates at
their time bound. Stale search recordings are checked in full against archived
catalog evidence whose snapshot digest is recalculated with the published
catalog digest contract. The current published catalog bytes must also match
their digest sidecar before the harness parses them.
