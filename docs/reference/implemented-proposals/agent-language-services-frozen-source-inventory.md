---
role: implementation-record
authority: supporting
update-when: The agent-language-services frozen source inventory, lifecycle artifacts, validator, acceptance corpus, or post-bootstrap immutability guard is superseded.
---

# Agent Language Services Frozen Source Inventory

The frozen source inventory slice of the agent-language-services lifecycle
migration is complete. The umbrella proposal remains in place, and the frozen
artifacts now provide the verification input for the separate content
migration PR.

## Completion Evidence

- The lifecycle artifact route is
  [Agent Language Services Lifecycle Artifacts](../agent-language-services-lifecycle/README.md).
- The tracked target provenance is
  [target-provenance.json](../agent-language-services-lifecycle/target-provenance.json).
- The source-universe contract, frozen inventory, lifecycle manifest, and
  migration-ledger schema live under
  [agent-language-services-lifecycle/](../agent-language-services-lifecycle/README.md).
- The positive ledger fixture and injected rejection fixtures live under
  [ledger-fixtures/](../agent-language-services-lifecycle/ledger-fixtures/valid-ledger.json).
- The local artifact validator is:

```sh
node workflow-scripts/check-agent-language-services-lifecycle.mjs validate
```

- The local test command is:

```sh
node --test workflow-scripts/check-agent-language-services-lifecycle.test.mjs
```

- The documentation-validation workflow invokes the range-aware lifecycle
  guard for pull requests and pushes. When a pull request contains both the
  review-gate and frozen-inventory commits, the guard accepts it only if the
  commit history preserves a valid `G1` checkpoint before the frozen artifact
  bootstrap.

## Boundary

This record closes only the frozen inventory slice. It does not perform the
production content migration, add the production migration ledger, implement
MCP JSONL assertions, change executable MCP cases, or alter toolchain semantic
baselines.
