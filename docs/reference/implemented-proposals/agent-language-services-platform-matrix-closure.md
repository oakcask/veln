---
role: implementation-record
authority: supporting
update-when: The agent-language-services closed client-platform matrix, compatibility field identities, lifecycle source-universe prerequisite, or validation evidence is superseded.
---

# Agent Language Services Platform Matrix Closure

The agent-language-services proposal now closes the plugin client-platform
membership needed before the lifecycle source universe can be frozen.

## Completion Evidence

- `docs/proposals/agent-language-services.md` contains the literal
  client-platform table with exactly:
  `codex/x86_64-unknown-linux-gnu` and
  `claude-code/x86_64-unknown-linux-gnu`.
- The same proposal names the compatibility fields that future
  `compatibility.toml` rows must pin: client, platform, host build,
  manifest-schema revision, validator version, validator digest, and the
  required Veln, MCP, LSP, language-service, and reference-schema contracts.
- `docs/reference/agent-language-services-lifecycle/source-universe.json` and
  `docs/reference/agent-language-services-lifecycle/frozen-inventory.json`
  preserve the closed plugin compatibility cells as reviewed identity sets.
- `workflow-scripts/check-agent-language-services-lifecycle.mjs validate`
  checks the frozen identity set, source digests, inventory coverage, lifecycle
  manifest, migration-ledger schema corpus, and diff-scope prerequisite gate.
- `workflow-scripts/check-agent-language-services-lifecycle.test.mjs` rejects a
  missing plugin compatibility cell.

## Boundary

This closure records membership and field identity only. It does not add
plugin artifacts, executable MCP cases, client smoke tests, compatibility
values, host builds, schema revisions, validator versions, or integrity
digests.
