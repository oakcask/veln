---
role: proposal
update-when: The planned standard-library handler rejection or retained-state matrices or their executable evidence change.
---

# Standard-Library Handler Navigation

This proposal defines the remaining standard-library handler contract and
evidence work for S5 and S6. The current [Editor
Support](../specification/editor-support.md#lsp-navigation-formatting-and-rename)
and [MCP](../specification/mcp.md#saved-workspace-navigation) specifications own
the selected-project, package-location, and failure-state rules that constrain
this work.

## Remaining Acceptance And Verification

| Case | Input | Required observation | Planned evidence |
| --- | --- | --- | --- |
| S5: remaining standard-library rejection table | Use transitive, mismatched, unresolved, incomplete, additionally qualified, test-only, and wrong-kind standard-library inputs alongside an unrelated valid handler identity. | Every input keeps its existing empty or separate result without contaminating the unrelated valid identity. | Table-driven shared-selection and adapter negative tests. |
| S6: retained standard-library state | Keep a live MCP cursor for a standard-library handler, then reject a workspace capture and trigger resource-admission, position, and cursor failures. | Capture and admission failures publish no partial result or resource. Position and cursor failures preserve the prior declaration, workspace result, resources, and existing cursors at their original snapshots. | Focused MCP refresh, capacity, capture-failure, position, and cursor tests. |

Standard-library API companion tests are not applicable because the remaining
work changes language-service and adapter navigation contracts, not handler
runtime behavior. Identity and rejection matrices belong in
`veln-language-service`; adapters must not add independent handler scans.

## Related Work And Design Choice

[Veln PR #1981](https://github.com/oakcask/veln/pull/1981) added the shared
retained-package handler identity and LSP and MCP projection for direct
dependencies. Its separation between shared selection and adapter projection
is the design constraint for the remaining work: S5 and S6 must extend the
shared selection and retained-state boundaries instead of adding independent
adapter scans.

[Veln PR #1251](https://github.com/oakcask/veln/pull/1251) added retained
embedded-standard-library definitions and readable immutable virtual sources.
It establishes that the standard-library package revision and canonical
`veln-pkg:` locations are navigation identities.

[Veln PR #1781](https://github.com/oakcask/veln/pull/1781) publishes public
handlers such as `net::system` in standard-library package documentation while
excluding private handlers and implementation bodies. That catalog confirms a
durable public handler surface, but documentation publication neither selects
source applications nor proves definition and reference behavior.

External language-server designs do not determine Veln's retained package or
handler semantics. The repository's shared handler selection and
retained-resource paths therefore remain the relevant implementation route for
this bounded extension.

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

Implement S5 and S6 as separately reviewable contract and evidence
additions. Add each executable matrix before promoting its observations into
the current specifications. Do not infer identity from spelling or introduce
an adapter-specific handler scan. Remove this page and its Ready catalog entry
when both remaining rows are covered and the current specifications and
executable evidence own those boundaries.
