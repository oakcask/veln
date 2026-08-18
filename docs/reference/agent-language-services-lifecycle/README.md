---
role: reference
authority: normative
update-when: The agent-language-services frozen inventory, source-universe contract, lifecycle manifest, migration-ledger schema, target provenance, or lifecycle validator changes.
---

# Agent Language Services Lifecycle Artifacts

This directory contains the checked frozen-source artifact set for the first
agent-language-services lifecycle migration PR.

Read these artifacts before editing the migration validator:

- `target-provenance.json` binds the bootstrap target to the lifecycle
  migration subsection and prerequisite records.
- `source-universe.json` freezes the parsed source roots, source digests, and
  source-bound finite identities from `docs/proposals/agent-language-services.md`.
- `frozen-inventory.json` records the source text, heading, digest, lifecycle
  split, and Unicode-scalar spans for each frozen source item.
- `lifecycle-manifest.json` records the reviewed lifecycle class for every
  inventory leaf.
- `migration-ledger.schema.json` defines the structural contract consumed by
  the later migration ledger.
- `migration-ledger.schema-fixture.json` is the positive schema fixture used by
  the validator before the production migration ledger exists.

Run the local check with:

```text
node workflow-scripts/check-agent-language-services-lifecycle.mjs validate
```

The workflow script tests also inject invalid digests, inventory membership
errors, child-span errors, lifecycle errors, ledger schema and destination
errors, reviewed-authority mutation, ledger mapping errors, and bootstrap or
post-bootstrap diff-scope errors.
