---
role: implementation-record
authority: supporting
update-when: The agent-language-services closed client-platform matrix, its documentation validator, or its lifecycle prerequisite evidence is superseded or invalidated.
---

# Agent Language Services Platform Matrix Closure

## Outcome

The agent-plugin planning contract now has one literal closed client-platform
matrix. The lifecycle migration can freeze every plugin cell without relying
on an unnamed set of platforms.

This completed prerequisite defines a finite documentation contract. It does
not implement a plugin, run host validation, or claim that a client-platform
cell passes.

## Closed Contract

The authoritative table in
[Agent Language Services](../../proposals/agent-language-services.md#closed-client-platform-matrix)
contains exactly these ordered keys:

1. `codex/x86_64-unknown-linux-gnu`
2. `claude-code/x86_64-unknown-linux-gnu`

Each row records the client, platform, host build, manifest-schema revision,
validator version and integrity digest, and the required Veln, MCP, LSP,
language-service, and reference-schema contracts. Every value is an exact
nonempty literal. Every validator integrity digest contains exactly 64
lowercase hexadecimal digits.

The plugin requirements, Q21 evidence, Q22 totality evidence, and umbrella
completion rule all quantify over that table. They do not define a second
platform universe.

## Completion Evidence

The registered agent-language platform-matrix validator independently checks
the table and its references. Its focused tests cover every matrix
rejection class, exact key ordering and row count, compatibility-field
literal form, digest form, uniqueness, and the four required references.

The same validator contains a phase-aware closure diff guard. The guard applies
only when a base without the closed matrix transitions to a head with the exact
two-row matrix. Range tests prove that it retires for later documentation work
and rejects out-of-scope changes, protected-path renames, and Git type changes
during the closure transition.

The documentation-validation workflow runs the focused tests and validates the
repository matrix contract. Failure messages name the invalid row, reference,
or path, state the repair, and explain why the lifecycle inventory requires a
finite platform universe.

## Boundaries Preserved

- No `compatibility.toml`, plugin manifest, client installation case, or smoke
  test was added.
- No MCP or LSP fixture, harness behavior, semantic baseline, or Veln language
  behavior changed.
- The table does not claim that either cell is supported or host-validated.
- The lifecycle migration remains separate work and owns the later frozen
  source inventory and migration.

## Read When

- Reviewing why the lifecycle migration uses the exact two-row plugin matrix.
- Auditing the matrix validator or its one-transition diff guard.
- Checking why another client or platform requires a separate proposal.
