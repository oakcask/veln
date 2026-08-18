---
role: routing
update-when: The agent-language-services frozen lifecycle artifact set, validator registration, or acceptance corpus changes.
---

# Agent Language Services Lifecycle Artifacts

This directory contains the frozen verification artifacts for the
agent-language-services lifecycle migration. These artifacts are historical
inputs for the migration ledger and destination audit, not current language
behavior.

## Artifacts

- Source-universe contract: [source-universe.json](source-universe.json).
- Frozen inventory: [frozen-inventory.json](frozen-inventory.json).
- Lifecycle manifest: [lifecycle-manifest.json](lifecycle-manifest.json).
- Migration-ledger schema:
  [migration-ledger.schema.json](migration-ledger.schema.json).
- Target provenance: [target-provenance.json](target-provenance.json).
- Ledger acceptance corpus:
  [ledger-fixtures/valid-ledger.json](ledger-fixtures/valid-ledger.json) and
  invalid fixtures under `ledger-fixtures/invalid/`.

## Validation

Run the artifact validator with:

```sh
node workflow-scripts/check-agent-language-services-lifecycle.mjs validate
```

Run the acceptance and rejection tests with:

```sh
node --test workflow-scripts/check-agent-language-services-lifecycle.test.mjs
```

The documentation-validation workflow also invokes the range-aware lifecycle
guard. After the frozen inventory bootstrap, that guard rejects changes to the
frozen artifacts, validator, test corpus, and workflow registration.
