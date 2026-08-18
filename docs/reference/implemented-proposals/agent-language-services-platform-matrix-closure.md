---
role: implementation-record
update-when: The agent-language-services plugin clients, supported platforms, compatibility field identities, artifact-backed compatibility evidence, or lifecycle-migration source-universe prerequisite changes.
---

# Agent Language Services Platform Matrix Closure

## Summary

Replace the undefined agent-plugin platform set with a literal closed matrix.
The lifecycle migration cannot freeze every client-platform cell while the
umbrella proposal names clients but quantifies over an unnamed set of
“supported platforms.”

This is a documentation-contract prerequisite. It does not implement a plugin,
run host validation, or claim that a client-platform cell already passes.

## Correction Boundary

An implementation attempt showed that the previous completion rule was
self-contradictory. It required exact host, schema, validator, contract, and
digest values while excluding the artifacts and host validation that could
establish those values. Copying literal-looking values into a proposal, a
validator, and its tests proves only textual agreement.

This proposal now closes client-platform membership and compatibility-field
identity only. Later plugin implementation must derive compatibility values
from checked client and validator artifacts. That work remains incomplete and
cannot use this documentation-only closure as evidence that a value is valid.

## Completion State

This documentation-contract prerequisite is complete. The frozen source
inventory PR from
[Agent Language Services Lifecycle Migration](../../proposals/agent-language-services-lifecycle-migration.md)
is now selectable.

## Scope

The completed closure adds one literal client-platform membership table under
the unique visible heading `### Closed Client-Platform Matrix` in
`agent-language-services.md`. The table has exactly the columns `Client` and
`Platform`. Each data row declares one exact client and platform pair.

It also adds one separate table immediately under the unique visible heading
`#### Compatibility Field Identities` within that matrix section. The table
names the fields that every future `compatibility.toml` record selected by a
membership row must contain:

1. `client`
2. `platform`
3. `host-build`
4. `manifest-schema`
5. `validator-version`
6. `validator-integrity`
7. `veln-contract`
8. `mcp-contract`
9. `lsp-contract`
10. `language-service-contract`
11. `reference-schema-contract`

The initial table contains exactly these two ordered client-platform keys:

1. `codex/x86_64-unknown-linux-gnu`
2. `claude-code/x86_64-unknown-linux-gnu`

The closure did not select a different platform, add a third row, reorder the
keys, or use a range, wildcard, “all supported platforms,” or an unnamed
future row. A separate proposal must add another client or platform after this
closure.

The compatibility-field table contains field identities only. This closure did
not choose a host build, schema revision, validator version, contract revision,
or integrity digest. Later plugin work records each exact value only after an
authoritative client or validator artifact and its validation result exist.

The closure updated these exact semantic locations so each quantifier over
client-platform cells routes to the literal table:

| Reference ID | Section | Source claim |
| --- | --- | --- |
| `plugin-server-lifecycle` | `## Agent Plugin` | One server per active workspace root. |
| `plugin-installer-boundary` | `## Agent Plugin` | A later installer requires stable client installation contracts. |
| `plugin-compatibility-authority` | `## Agent Plugin` | `compatibility.toml` records compatibility contracts. |
| `plugin-native-validation` | `## Agent Plugin` | Each cell runs client-native validation. |
| `conformance-requirement-coverage` | `## Conformance Contract` | The manifest covers every normative client-platform cell. |
| `conformance-capability-membership` | `## Conformance Contract` | The v1 manifest closes plugin compatibility membership. |
| `q21-plugin-matrix` | `## Conformance Contract` | The `Q21 plugin matrix` evidence row. |
| `q22-gate-totality` | `## Conformance Contract` | The `Q22 gate totality` evidence row. |
| `umbrella-completion` | `## Conformance Contract` | The proposal completion paragraph. |
| `plugin-acceptance-completion` | `### Plugin` | The `Run the proposal completion gate` acceptance row. |

A requirement may say “every row in the closed table”; it may not introduce
another implicit platform set. The validator's independent expected-reference
fixture contains exactly these ten reference IDs and semantic locations. It
does not derive that expected set from the edited documents.

Each named umbrella block contains this exact visible source link, with its own
reference ID substituted:

```markdown
[Closed Client-Platform Matrix](#closed-client-platform-matrix "matrix-ref:<reference-id>")
```

The validator checks each link in its named visible Markdown block. A matching
link or phrase in another paragraph, table row, comment, code block, block
quote, image text, link destination, or link title does not satisfy the named
reference. This exact ten-link registry is the finite oracle; the validator
does not attempt to classify arbitrary natural-language synonyms.

## Checked Matrix Contract

The closure adds a documentation validator that extracts the membership and
field-identity tables independently from the later frozen-source inventory.
The validator records the ordered client-platform keys, exact row count, and
ordered field identities. It rejects:

- a missing or duplicate client-platform key;
- a membership or field-identity table hidden in an excluded Markdown context,
  displaced from its exact heading, or duplicated;
- an empty client or platform identifier;
- a range, wildcard, placeholder, or catch-all row;
- a missing, duplicate, reordered, or unexpected compatibility-field identity;
- a compatibility value or integrity digest added by this closure;
- a plugin, Q21, Q22, or completion reference to an unnamed platform set; and
- a checked row-count value that differs from the table.

The validator runs through the existing documentation-validation workflow.
Its failure output names the invalid row, field identity, reference, or path;
tells the maintainer to enumerate or restore the exact item; and explains that
the lifecycle inventory cannot prove finite coverage otherwise.

The closure diff guard is active only when the base revision lacks the closed
matrix and the head revision adds the exact two-row table. It retires after
that transition merges. The active guard permits exactly this path-operation
manifest, derived without Git rename or copy detection:

```text
M .github/workflows/workflow--test-scripts.yaml
M docs/proposals/README.md
M docs/proposals/agent-language-services-lifecycle-migration.md
D docs/proposals/agent-language-services-platform-matrix-closure.md
M docs/proposals/agent-language-services.md
M docs/reference/implemented-proposals/README.md
A docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md
M docs/specification/README.md
M docs/specification/mcp.md
M docs/specification/topic-map.md
A workflow-scripts/check-agent-language-platform-matrix.mjs
A workflow-scripts/check-agent-language-platform-matrix.test.mjs
```

Modified and deleted paths start as regular non-executable files. Added and
modified paths end as regular non-executable files. The guard rejects a missing
operation, extra path, different operation, executable-bit change, or Git type
change. It does not permit unrelated CI remediation. A range test proves that
an unrelated documentation change after closure does not inherit the closure
allowlist. Separate tests reject a protected-path move represented as deletion
and addition other than the required proposal-to-implementation-record archive
operation, and a regular file changed to another Git type.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Enumerate the intended plugin compatibility set. | One visible table under the exact matrix heading contains `codex/x86_64-unknown-linux-gnu` followed by `claude-code/x86_64-unknown-linux-gnu` and no other or implicit membership. | Independent expected-key fixture, exact row count of two, and duplicate, displaced, comment, code-block, block-quote, and link-metadata table mutations. |
| Preserve one compatibility contract shape per cell. | One visible table under the exact field-identity heading contains the eleven ordered compatibility-field identities exactly once and contains no values. Every future record selected by a membership row uses that shape. | Missing, duplicate, reordered, unexpected, value-bearing, displaced, and hidden field-table mutations. |
| Keep cell identity unique and literal. | Empty, duplicate, ranged, wildcard, placeholder, and catch-all client-platform keys fail. | Injected rejection cases for each invalid key class. |
| Close all references over the same set. | Each of the ten named visible Markdown blocks contains its exact titled matrix link. A hidden, displaced, mistitled, or wrong-destination link does not satisfy the contract. | Per-location missing, displaced, duplicate, comment, code-block, block-quote, image, wrong-title, and wrong-destination mutations. |
| Keep the prerequisite documentation-only. | The matrix-addition transition does not add plugin artifacts, executable MCP cases, harness changes, semantic baselines, or unrelated CI remediation, and its allowlist retires after merge. | Phase-aware diff-scope cases for the matrix transition, an extra path, a later unrelated docs change, a protected-path rename, and a Git type change. |

## Non-Goals

- Choosing or implementing the future plugin packaging mechanism.
- Claiming support for a client-platform cell before its pinned validation
  passes.
- Choosing compatibility values before checked client and validator artifacts
  establish them.
- Adding `compatibility.toml`, client manifests, MCP or LSP smoke tests, or an
  installer.
- Freezing the broader agent-language-services source inventory.
- Adding another client or platform after the exact two-cell set closes.

## Completion Rule

This proposal completed because all five acceptance rows pass and all ten
registered locations contain their exact titled matrix links. The
implementation record preserves the Correction Boundary rationale. The
completed record now lives outside `docs/proposals/`, so the lifecycle
migration's frozen-source-inventory PR can be selected.
