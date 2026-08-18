---
role: reference
authority: supporting
update-when: The agent-language-services frozen source inventory artifacts, validator, migration-ledger schema, or lifecycle-migration phase boundary changes.
---

# Agent Language Services Frozen Inventory

This page routes the first lifecycle-migration PR artifacts for
`docs/proposals/agent-language-services.md`.

The checked artifacts are:

- [agent-language-services-source-universe.json](agent-language-services-source-universe.json)
- [agent-language-services-frozen-inventory.json](agent-language-services-frozen-inventory.json)
- [agent-language-services-lifecycle-manifest.json](agent-language-services-lifecycle-manifest.json)
- [agent-language-services-migration-ledger.schema.json](agent-language-services-migration-ledger.schema.json)

Run the local check with:

```sh
node workflow-scripts/check-agent-language-services-inventory.mjs --skip-diff-scope
```

The CI documentation-validation route runs the same validator and its mutation
tests. The bootstrap diff-scope guard is active only while the frozen
inventory is first added.
