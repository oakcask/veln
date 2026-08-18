---
role: reference
authority: normative
update-when: The agent-language-services frozen lifecycle artifacts, migration ledger schema, validator, or documentation-validation workflow registration changes.
---

# Agent Language Services Lifecycle Artifacts

The JSON files in this directory are the reviewed frozen source universe for
`docs/proposals/agent-language-services.md`.

Run this local check before changing these artifacts:

```sh
node workflow-scripts/check-agent-language-services-lifecycle.mjs validate
```

The validator checks the source digests, inventory coverage, parent and child
span partitioning, reviewed lifecycle manifest, migration-ledger schema, and
the first-PR diff-scope guard.
