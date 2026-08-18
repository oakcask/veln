---
role: implementation-record
authority: supporting
update-when: The agent-language-services plugin client-platform matrix, compatibility fields, or frozen source-universe prerequisite changes.
---

# Agent Language Services Platform Matrix Closure

The agent-language-services umbrella proposal now contains the closed v1
client-platform matrix for plugin compatibility. The checked keys are:

- `codex/x86_64-unknown-linux-gnu`
- `claude-code/x86_64-unknown-linux-gnu`

Each row names the client, platform, host build, manifest-schema revision,
validator version, validator digest, and required Veln, MCP, LSP,
language-service, and reference-schema contracts. The documentation validator
checks the exact key list, row count, required fields, digest shape, and the
absence of unbound supported-platform references before the lifecycle migration
freezes the source universe.

The completed closure lets the lifecycle migration use the matrix rows as
finite source identities. Adding another client or platform requires a new
proposal that changes the closed key list explicitly.
