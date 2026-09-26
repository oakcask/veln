---
role: proposal
update-when: The planned agent language-reference skill routing, published language-reference resource contract, or repository documentation authority changes.
---

# Agent Language-Reference Skill Routing

## Outcome

Provide one client-neutral agent skill named `veln-language`. Its canonical
source belongs in the repository's `.agents` skill directory. The skill
distinguishes language questions from repository-change tasks. It uses the
published language reference for language questions and the repository
documentation route for changes to the implementation. This slice does not
depend on client plugin packaging.

## Acceptance Model

| Input state | Observable result | Planned evidence |
| --- | --- | --- |
| A language question matches one or more published reference topics. | The skill uses language-scoped documentation search, reads the matching topic resource, and limits its answer to claims supported by the returned resource. The answer identifies the resource URI used. | Checked skill scenario with a known topic, the expected search and read requests, and an answer-source assertion. |
| A language question has no matching published topic. | The skill reports that the published reference has no matching topic. It does not substitute proposal text or an inferred language rule. | Checked no-match scenario that rejects unsupported behavior claims and proposal fallback. |
| Documentation search is unavailable, a selected topic cannot be read, or its URI returns `resource_not_found` because the snapshot digest is stale. | The skill reports which reference operation or artifact is unavailable or stale and does not answer the language question from memory. A previous successful result remains unchanged. | Checked unavailable-search, unavailable-topic, stale-snapshot-URI, and state-preservation scenarios. |
| The request asks to inspect or change the Veln repository. | The skill starts at `docs/README.md`, follows the smallest current specification or proposal route for the task, and does not treat the published language reference as repository implementation authority. | Checked repository-task scenarios for current behavior, proposal selection, and an unknown repository topic. |

The scenario harness must use recorded tool results so it can verify request
selection, answer provenance, failure behavior, and unchanged prior results
without network access or a particular agent client.

## Boundaries

- The skill source and its scenario contract are shared assets. Codex and
  Claude Code manifests, installation flows, and client-specific adapters
  remain in the client-plugin scope of the umbrella proposal.
- The skill does not expand the language-reference catalog, MCP resource
  contract, or repository documentation authority.
- The skill does not use `docs/proposals/` to answer questions about current
  language behavior.
- An unavailable or stale reference is a bounded failure. It does not permit a
  best-effort language answer from model memory.

## Completion

This proposal is complete when the client-neutral skill asset and all rows in
the acceptance model pass in the repository-local scenario harness, and the
skill's maintenance route names the published language-reference contract and
repository documentation authority that can make it stale.
