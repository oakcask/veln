---
role: proposal
update-when: The planned standard-library handler owner, revision, source, project, rejection, or retained-state matrices or their executable evidence change.
---

# Standard-Library Handler Navigation

Current standard-library handler selection and adapter projection are specified
by [Editor Support](../specification/editor-support.md#lsp-navigation-formatting-and-rename)
and [MCP](../specification/mcp.md#saved-workspace-navigation). Checked shared,
LSP, MCP transcript, and cross-adapter evidence owns the implemented S1 and S2
surface. This proposal retains the remaining contract detail and evidence work
for S3 through S6. Current specifications continue to own the general
selected-project, package-location, and failure-state rules that apply to
standard-library handlers. The remaining rows track standard-library-specific
matrix coverage and any contract detail not yet adopted by those
specifications.

## Remaining Acceptance And Verification

The acceptance rows below describe the remaining standard-library-specific
matrix coverage. Current specifications and focused tests already own the S1
and S2 guards for private, non-exported, invalid-cased, duplicate, and
recovered declarations, repeated and recovered imports, and standard-library
alias collisions with implicit or explicit workspace import aliases. Those
guards are not requirements of this proposal. The rejection inputs listed in
S5 remain planned work.

| Case | Input | Required observation | Planned evidence |
| --- | --- | --- | --- |
| S3: owner and revision isolation | Provide equal module and handler names in a dependency and workspace module, then repeat the selection across sequential snapshots with different retained standard-library revisions. | Each selection retains its exact `std` owner and revision. No location is rewritten to another owner or stale revision. | Shared-language-service collision tests plus adapter snapshot tests. |
| S4: source and project boundaries | Place equal applications in standard-library bodies, a different workspace project, comments, and strings, then select each workspace project independently. | Each result contains only applications from its selected saved project and exposes no standard-library build path. | MCP project-isolation tests plus focused LSP virtual-document cases. |
| S5: remaining rejection table | Use transitive, mismatched, unresolved, incomplete, additionally qualified, test-only, and wrong-kind inputs alongside an unrelated valid handler identity. | Every input keeps its existing empty or separate result without contaminating the unrelated valid identity. | Table-driven shared-selection and adapter negative tests. |
| S6: retained state | Keep a live MCP cursor, then reject a workspace capture and trigger resource-admission, position, and cursor failures. | Capture and admission failures publish no partial result or resource. Position and cursor failures preserve the prior declaration, workspace result, resources, and existing cursors at their original snapshots. | Focused MCP refresh, capacity, capture-failure, position, and cursor tests. |

Standard-library API companion tests are not applicable because the remaining
work changes language-service and adapter navigation contracts, not handler
runtime behavior. Identity and rejection matrices belong in
`veln-language-service`; adapters must not add independent handler scans.

## Related Work And Design Choice

[Veln PR #1981](https://github.com/oakcask/veln/pull/1981) added the shared
retained-package handler identity and LSP and MCP projection for direct
dependencies. The current implementation now reuses those handler traversal,
package declaration, adapter, and failure-state boundaries for the implemented
standard-library selection. That reuse supports S1 and S2, but it does not
establish the remaining standard-library-specific isolation, rejection, or
retained-state observations planned below.

[Veln PR #1251](https://github.com/oakcask/veln/pull/1251) added retained
embedded-standard-library definitions and readable immutable virtual sources.
It establishes that the standard-library package revision and canonical
`veln-pkg:` locations are navigation identities. The current specifications
and executable evidence, rather than that earlier change, establish handler
eligibility and application membership.

[Veln PR #1781](https://github.com/oakcask/veln/pull/1781) publishes public
handlers such as `net::system` in standard-library package documentation while
excluding private handlers and implementation bodies. That catalog confirms a
durable public handler surface, but documentation publication neither selects
source applications nor proves definition and reference behavior.

The current
[Editor Support](../specification/editor-support.md#lsp-navigation-formatting-and-rename)
and [MCP](../specification/mcp.md#saved-workspace-navigation) specifications remain
authoritative for implemented behavior. Reusing their shared handler selection
and retained-resource paths avoids an adapter-specific standard-library
identity. External language-server designs do not determine Veln's retained
package or handler semantics, so repository implementation history and current
contracts are the relevant related work for this bounded extension.

## Non-Goals

- Do not add transitive-dependency handler navigation.
- Do not add package or standard-library effect or effect-operation navigation.
- Do not add handler rename, completion, hover, signature help, or symbols.
- Do not include standard-library implementation applications or cross-project
  references.
- Do not admit test-only standard-library modules into production navigation.
- Do not change package capture, exports, source-resource URIs, location
  ordering, cursors, or protocol schemas.

## Implementation And Specification Route

Implement S3 through S6 as separately reviewable contract and evidence
additions. Add each executable matrix before promoting its observations into
the current specifications. Do not infer identity from spelling or introduce
an adapter-specific handler scan. Remove this page and its Ready catalog entry
when all four remaining rows are covered and the current specifications and
executable evidence own those boundaries.
