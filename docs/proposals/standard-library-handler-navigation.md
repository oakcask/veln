---
role: proposal
update-when: The planned standard-library handler retained-state matrix or its executable evidence changes.
---

# Standard-Library Handler Navigation

This proposal defines the remaining standard-library handler retained-state
contract and evidence work for S6. The current [Editor
Support](../specification/editor-support.md#lsp-navigation-formatting-and-rename)
and [MCP](../specification/mcp.md#saved-workspace-navigation) specifications own
the selected-project, package-location, and failure-state rules that constrain
this work.

## Remaining Acceptance And Verification

| Case | Input | Required observation | Planned evidence |
| --- | --- | --- | --- |
| S6: retained standard-library state | Keep a live MCP cursor for a standard-library handler, then reject a workspace capture and trigger resource-admission, position, and cursor failures. | Capture and admission failures publish no partial result or resource. Position and cursor failures preserve the prior declaration, workspace result, resources, and existing cursors at their original snapshots. | Focused MCP refresh, capacity, capture-failure, position, and cursor tests. |

Standard-library API companion tests are not applicable because the remaining
work changes MCP retained navigation state, not handler runtime behavior. The
retained-state matrix belongs at the MCP adapter boundary and must continue to
consume shared handler selection instead of adding an independent handler scan.

## Related Work And Design Choice

[Veln PR #1981](https://github.com/oakcask/veln/pull/1981) added the shared
retained-package handler identity and LSP and MCP projection for direct
dependencies. Its separation between shared selection and adapter projection
is the design constraint for the remaining work: S6 must extend the
retained-state boundary instead of adding an independent adapter scan.

[Veln PR #1251](https://github.com/oakcask/veln/pull/1251) added retained
embedded-standard-library definitions and readable immutable virtual sources.
It establishes that the standard-library package revision and canonical
`veln-pkg:` locations are navigation identities.

[Veln PR #1781](https://github.com/oakcask/veln/pull/1781) publishes public
handlers such as `net::system` in standard-library package documentation while
excluding private handlers and implementation bodies. That catalog confirms a
durable public handler surface, but documentation publication neither selects
source applications nor proves retained definition and reference state.

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

Implement S6 as a reviewable contract and evidence addition. Add its executable
matrix before promoting its observations into the current specifications. Do
not infer identity from spelling or introduce an adapter-specific handler scan.
Remove this page and its Ready catalog entry when the remaining row is covered
and the current specifications and executable evidence own that boundary.
