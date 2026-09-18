---
role: proposal
update-when: Direct-dependency schema-alias chain eligibility, reference identity, saved navigation scope, or cross-adapter evidence changes.
---

# Direct-Dependency Schema-Alias Chain References

## Outcome And Readiness

Allow a consumer to find references to a public schema alias whose target is
another public schema alias in the same direct dependency. Preserve the
selected alias's identity instead of merging all aliases of the final schema.
This is graph-based target resolution, not an additional fixed chain length.

The saved package capture, qualified import resolution, schema composition
and operation reference collection, and LSP/MCP adapters already exist.
[Editor Support](../specification/editor-support.md) and
[MCP Navigation](../specification/mcp.md) specify their current boundary:
an alias must directly target a schema, and chains remain unsupported.
The `alias chain` rejection case in
`crates/veln-language-service/src/tests/dependencies_schema_references.rs`
and the direct-schema eligibility check in
`crates/veln-language-service/src/navigation/package_schemas.rs` demonstrate
the remaining gap.

This slice needs no new source syntax, MCP tool, package origin, or plugin
contract. It is independently selectable from the
[agent-language-services inventory](agent-language-services.md).

## Contract

A qualified consumer composition or operation leaf can select a public alias
in an exported source of a retained direct dependency when its target path
forms a finite acyclic chain ending at a unique public schema in that same
dependency. Every alias and the terminal schema must have valid casing, be
public, and be declared in an exported source. Each hop uses the existing
direct-target lookup rules in the declaring alias's explicit module, including
imports shared by package sources with that module identity. Consumer imports
do not resolve package-internal target paths.

At each hop, a duplicate, recovered declaration, schema/alias collision,
invalid or ambiguous target import, missing target, or wrong-kind target makes
the selected chain ineligible. A same-spelled declaration in an unrelated
namespace does not itself invalidate a schema target. Bare targets resolve
within the alias module; qualified targets use full written module paths or
unique implicit leaf imports. No hop may leave the retained dependency.

Results contain only the selected project's saved consumer leaves that name
the selected alias: direct fields, valid `Repeat` and array payloads, `decode`,
and `encode`. Chain-intermediate aliases, sibling aliases, and the terminal
schema keep separate reference identities. Package source occurrences and
declarations are excluded, including when LSP requests declaration inclusion.
Existing ordering, deduplication, coordinates, pagination, scope, and snapshot
failure contracts remain in force. Ineligible selections return successful
empty reference results under the existing scope contract.

Definition and rename support do not expand. Workspace alias chains,
standard-library aliases, transitive dependencies, cross-package targets,
bare imported consumer aliases, and recovery or casing-neutral navigation
remain outside this slice.

## Acceptance Model

The following rows are planned evidence, not passing implementation claims.
The table is the acceptance authority until checked cases replace it; prose
alone cannot demonstrate the proposed navigation results.

| Input or event | Required observation | Planned evidence |
| --- | --- | --- |
| One dependency exports `Alias = Middle`, `Middle = Packet`, and schema `Packet`; the consumer uses all three plus a sibling alias. | Selecting each consumer alias leaf returns exactly that alias's consumer composition and operation leaves; no target or sibling union occurs. | Shared navigation exact-location test and paired LSP/MCP executable cases. |
| A chain crosses exported modules within one dependency using valid bare, full qualified, and implicit leaf target paths, including imports in another source with the same explicit module. | Every hop resolves from its declaring module; consumer imports do not change the chain. | Shared target-resolution table. |
| Two dependencies export the same module and alias spellings. | Each selected alias returns only its own consumer references. | Shared identity case and paired adapter cases. |
| Any intermediate alias or terminal schema is private, non-exported, invalid-cased, missing, duplicated, recovered, ambiguous, or of the wrong kind; or a hop leaves the dependency. | The selected chain returns no references and never falls back to a same-spelled schema. An unrelated valid chain remains usable. | Shared negative matrix, covering failures beyond the first hop, and MCP empty-result case. |
| An alias targets itself, a cycle of aliases, or a chain that reaches a cycle. | Reference lookup terminates with an empty set for the affected selection; disjoint valid chains still resolve. | Shared cycle and isolation cases. |
| Consumer imports are duplicated, recovered, or have colliding implicit leaf aliases. | Existing direct-dependency import visibility and exact-written-path precedence are preserved. | Extend the shared import-blocker matrix to a chain-backed alias. |
| The saved project includes non-BMP text, a descendant manifest, comments, strings, invalid repeated-payload counts, and recovered leaves. | Results include only eligible owned-source leaves with exact ranges. LSP with either declaration policy and all negotiated encodings agrees with concatenated MCP pages after coordinate normalization. | Paired executable cases under `examples/specification/lsp/` and `examples/specification/mcp/`, plus adapter coordinate tests. |
| Saved capture changes until retry exhaustion during a chain reference request. | MCP returns `snapshot_changed` without partial references or package publication; prior selection and retained resources remain usable. | Extend MCP saved-capture failure tests. |
| A chain-backed leaf is used for definition, prepare-rename, or rename. | Existing unsupported results remain unchanged; reference support grants no edit capability. | Paired navigation and LSP no-edit assertions. |
| A generated family contains long chains, many aliases sharing a suffix, and disconnected cycles. | All finite valid chains resolve without a fixed hop limit; cycles terminate and repeated suffixes do not cause repeated whole-graph expansion. | Bounded shared-service work-counter tests over increasing graph sizes, extending the existing schema-alias indexing guards. |

## Completion

Implement the rows in the shared service and both adapters, with explicit
expected URI/range sets rather than parity alone. Extend the existing
`references-dependency-schema-alias` executable cases or add focused companion
cases. Keep rejection coverage for the excluded origins and scopes.

Run the affected language-service and MCP tests and LSP/MCP specification
harness cases through `scripts/agent-test`, and perform the repository's
performance-regression audit for the generated graph cases. No new conformance
manifest or plugin package is a prerequisite for this bounded slice.

After the acceptance evidence passes, update the smallest matching sections
of Editor Support and MCP Navigation, remove this proposal and its Ready
entry, and update the umbrella's remaining-work routes. Do not promote planned
chain behavior to current specification before those checks pass.
