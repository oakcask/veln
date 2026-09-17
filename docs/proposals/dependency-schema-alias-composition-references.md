---
role: proposal
update-when: Direct-dependency schema-alias composition reference selection, alias eligibility, import resolution, or planned LSP and MCP evidence changes.
---

# Direct-Dependency Schema-Alias Composition References

## Scope And Readiness

Let a consumer find all saved project uses of an eligible direct-dependency
schema alias, including composition fields as well as decode and encode uses.
This is a bounded slice of the
[agent language-services inventory](agent-language-services.md).

The prerequisites are implemented: direct-dependency schema composition,
workspace schema-alias composition, and direct-dependency schema-alias
operation references. Their current boundaries are specified by
[Editor Support](../specification/editor-support.md) and
[MCP Navigation](../specification/mcp.md). The existing package alias reference
collector includes operation leaves but restricts composition references to
workspace aliases. The paired `references-dependency-schema-alias` executable
cases currently require package-alias composition selection to be empty.
This proposal changes that observable boundary.

Reuse the current direct-dependency alias eligibility and target-resolution
contract, including exported-source visibility, unique public direct schema
targets in the same dependency, same-module and qualified cross-module targets,
and invalid-import, collision, recovery, and casing exclusions. No prerequisite
requires alias-chain traversal, standard-library aliases, pagination, or MCP
rename. The two identifier-casing proposals remain independently blocked by
missing explicit import-alias syntax and a missing MCP rename contract.

## Acceptance Model

This table specifies planned behavior. Its evidence is not yet implemented.
Use an eligible public alias `PacketCodec` and its direct public target
`Packet` in a retained direct dependency. Include a second alias of `Packet`
and a same-spelled alias in a different dependency to establish identity.

| Input or selection | Required observation | Planned evidence |
| --- | --- | --- |
| Select the alias leaf in a direct composition field, a valid `Repeat` payload, an array payload, decode, or encode. | Every selection returns the same union of those composition and operation leaves for that dependency and alias declaration. | Shared navigation table and paired MCP/LSP executable cases with exact ranges. |
| Use full written module paths and unique implicit leaf import aliases. | Both spellings resolve under the existing composition import rules; exact imports take precedence, conflicting exact imports and ambiguous implicit aliases select no identity. | Shared import table, including workspace/package collisions and both import orders. |
| Use a bare imported alias name or select an import token or module qualifier. | Return no references for that unsupported selection. | Negative selection rows in the paired cases. |
| Select the target schema, a sibling alias, or a same-spelled alias from another dependency. | Each result contains only its own identity's uses; none absorbs the selected alias's composition leaves. | Shared identity cases and exact MCP result assertions. |
| Use an eligible alias with a qualified cross-module target in its dependency. | Composition and operation selections return the same alias-specific union as for a same-module target. | Shared and paired cross-module target cases. |
| Use an alias chain, private or non-exported target, external-package target, invalid-cased or ambiguous declaration, recovered declaration/import/leaf, or invalid repeat count. | The unsupported leaf contributes no references and its selection returns an empty result; it cannot fall back to a same-spelled schema. | Shared boundary table and MCP boundary specification cases. |
| Include comments, strings, ordinary types, package-source uses, and uses under an unselected descendant project. | None enters the selected project's reference result. | Exact result fixtures with lexical noise and descendant ownership boundaries. |
| Request LSP references with either declaration-inclusion value and MCP references over identical saved files containing non-BMP text. | Both adapters return identical normalized, sorted, deduplicated workspace locations; neither includes a package declaration. MCP retains project-wide scope and existing scalar coordinates. | Paired executable cases with URI/range normalization and both LSP declaration policies. |
| Change dependency inputs during capture until the existing retry limit is exhausted. | MCP returns `snapshot_changed` without partial references or scope and preserves the previous selection and already published resources. | Extend the existing capture-failure harness with composition selection. |
| Request definition or rename on the newly supported alias composition leaf. | Preserve the existing unsupported definition and rename results. | LSP null definition/prepare-rename and empty rename-edit assertions; MCP definition remains empty. |

## Verification And Completion

Extend the existing shared dependency-schema reference tests, MCP dependency
schema and capture-failure tests, and paired `references-dependency-schema-alias`
cases under `examples/specification/`. Update their former empty-composition
assertions to the union above while retaining the independent negative rows.
The existing `codec-schema-references` check case supplies source-semantics
evidence; add a checked source case if a new fixture shape lacks such evidence.
The specification harness and local verification commands are routed by
[Toolchain Test Harness](../reference/toolchain-test-harness.md).

At implementation time, run focused shared-service and MCP tests and both
adapter specification cases. Apply the repository performance audit to the
analysis change; preserve indexed alias eligibility rather than adding an
unbounded per-reference declaration scan.

Completion requires passing evidence for every table row, current behavior
updates in Editor Support and MCP Navigation, removal of this proposal and its
Ready entry, and an updated umbrella inventory. Alias chains, other package
origins, package-source result locations, pagination, recovery navigation, and
casing-neutral navigation remain separate work.
