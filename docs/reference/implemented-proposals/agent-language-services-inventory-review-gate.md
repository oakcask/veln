---
role: implementation-record
authority: supporting
update-when: The agent-language-services target provenance, reviewed source-decision authority, frozen-inventory bootstrap contract, or lifecycle validator acceptance evidence is superseded.
---

# Agent Language Services Inventory Review Gate

This record closes the inventory review gate that previously blocked the
agent-language-services lifecycle migration.

The completed gate supplies two tracked authorities for the frozen inventory
bootstrap:

- reviewed source decisions:
  [../agent-language-services-lifecycle-review/source-decisions.json](../agent-language-services-lifecycle-review/source-decisions.json);
- PR-visible target provenance:
  [../agent-language-services-lifecycle/target-provenance.json](../agent-language-services-lifecycle/target-provenance.json).

The bootstrap artifact set is checked by:

```text
node workflow-scripts/check-agent-language-services-lifecycle.mjs validate
node --test workflow-scripts/check-agent-language-services-lifecycle.test.mjs
```

The test corpus covers positive validation plus injected failures for digest
drift, missing and duplicate inventory items, missing child records, span gaps,
span overlap, out-of-range child spans, mixed lifecycle leaves, direct parent
ledger mappings, missing and duplicate ledger leaves, wildcard and range ledger
entries, invalid removed conformance leaves, and bootstrap diff-scope changes
outside the first-PR allowlist.
