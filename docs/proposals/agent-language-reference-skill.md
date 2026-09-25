---
role: proposal
update-when: The canonical veln-language skill contract or its checked request-selection rules change.
---

# Canonical Agent Language Routing Contract

## Outcome

Align the canonical `veln-language` skill contract with the implemented
request-selection rules in the offline harness and the current agent language
routing specification.

## Remaining acceptance model

| Input state | Observable result | Planned evidence |
| --- | --- | --- |
| The canonical skill is replayed against the recorded scenarios. | Its top-level selection rule routes requested inspection, test, or change actions and explicit repository subjects to repository documentation. | The canonical replay loads the contract and passes all recorded scenarios. |
| A request contains a repository target or an action word. | The canonical contract distinguishes a requested subject or action from a term definition or incidental mention. | The canonical replay passes the checked positive and negative request-selection cases for every explicit target and intent verb. |

## Boundaries

- The harness, fixtures, and current specification already implement these
  rules. This proposal covers only the stale canonical skill contract.
- Client manifests, installation flows, adapters, and the published
  language-reference and MCP contracts remain outside this work.

## Completion

This proposal is complete when the canonical skill includes the checked
explicit-subject and requested-action rules, states the same top-level routing
contract, and passes the complete offline scenario suite. Remove this page and
its Ready entry only after that evidence passes.
