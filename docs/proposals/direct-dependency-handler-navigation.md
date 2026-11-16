---
role: proposal
update-when: The planned direct-dependency handler selection, package identity, reference scope, adapter projection, exclusions, or acceptance evidence changes.
---

# Direct-Dependency Handler Navigation

Navigate a public handler from a retained direct dependency through complete
qualified applications in the selected workspace project. The shared language
service already indexes package-owned handler declarations and workspace
handler applications, while current handler navigation deliberately stops at
workspace owners. This proposal extends that existing identity boundary to one
bounded package symbol class.

This proposal is ready for implementation. It changes shared navigation and
its existing LSP and MCP projections. It does not change handler syntax,
dependency resolution, package exports, effect execution, or either protocol.

## Scope And Identity

An eligible handler has one valid-cased, unrecovered public declaration in an
exported module of a retained direct dependency. A saved workspace source
imports that exact package and module with a parse-clean `use` declaration. A
complete `handle Body with module::handler(arguments)` application through
that import selects the package, module, and handler-name identity.

The handler-name leaf is the selected range. The module qualifier and import
declaration retain their existing identities. Every eligible application in
the selected project that resolves to the same package identity, module, and
handler declaration joins one reference set. Equal spelling in another
package, workspace module, symbol class, or dependency revision does not join
that identity.

The dependency declaration uses its retained immutable `veln-pkg:` location
for definition and MCP declaration inclusion. Workspace applications use their
ordinary workspace locations. Package source bodies do not contribute
application references; the selected saved workspace project is the reference
universe.

## Observable Contract

Selecting an eligible workspace application returns the retained dependency
handler declaration as definition and the selected project's deterministic
workspace application locations as references.

LSP and MCP project the shared result through their existing coordinate,
ordering, pagination, stable-snapshot, and failure contracts. LSP references
contain only workspace application locations, including when
`includeDeclaration` is true. A project-scoped MCP request with
`include_declaration: true` adds the canonical package declaration once before
sorting and pagination; false omits it. Neither adapter converts that
declaration to a workspace `file:` location. Handler rename remains unsupported
and produces no edits.

The following selection decisions apply:

| Input | Result |
| --- | --- |
| Public unrecovered handler in an exported retained direct-dependency module, reached through its exact import | Shared package handler definition and selected-project reference set. |
| Same handler reached by every accepted direct-dependency source form | The same observable identity boundary and location kinds; source materialization details are not exposed. |
| Private, non-exported, invalid-cased, duplicate, or recovered dependency handler | Unsupported selection with the adapter's existing valid-but-empty result. |
| Handler from the standard library, a transitive dependency, a rejected dependency capture, or a different dependency revision | Unsupported selection; no fallback to equal spelling in retained state. |
| Ambiguous, recovered, mismatched-package, unresolved, incomplete, additionally qualified, or wrong-kind application | Unsupported selection; no inferred handler identity. |
| Workspace handler with the same module and name | Its existing workspace identity remains separate. |

An unsupported handler leaf does not fall back to a function, module value,
type, or other symbol at the same path. Comments, strings, dependency
implementation applications, and applications in another selected project do
not enter the result. An unrelated parse error does not remove a complete,
parse-clean eligible application.

MCP capture, resource-admission, lookup, position, and cursor failures publish
no partial locations and preserve the accepted workspace, retained resources,
prior results, and existing cursors. A rejected dependency refresh cannot
replace the retained declaration or attach new workspace applications to its
identity. LSP continues to read its accepted project snapshot and retained
virtual source catalog.

## Acceptance And Verification

All evidence in this section is planned. Existing workspace-handler and
package-function cases establish reusable boundaries; they do not prove the
proposed package-handler behavior.

| Case | Input | Required observation | Planned evidence |
| --- | --- | --- | --- |
| D1: shared selection | Select one public dependency handler from several complete qualified applications in one saved project. | Every selection returns the same canonical package definition and workspace application set; ranges cover only handler names. | Focused language-service identity tests plus LSP definition and references tests. |
| D2: adapter agreement | Request definition and declaration-inclusive and declaration-excluding references through LSP and MCP. | After coordinate conversion, both adapters project the same workspace locations. LSP omits the package declaration; project-scoped MCP includes it exactly once only when requested and preserves its ordering and pagination contract. | Checked MCP transcript and saved-navigation cross-adapter coverage. |
| D3: package-owner isolation | Provide equal module and handler names in two direct dependencies and a workspace module, then refresh one dependency to a new captured revision. | Each selection retains its exact package owner, and each accepted snapshot retains its revision. No location is rewritten to another owner or stale revision. | Shared-language-service collision tests plus adapter capture and refresh tests. |
| D4: source and project boundaries | Reach the handler through each accepted direct-dependency source form and place equal applications in dependency bodies, another selected project, comments, and strings. | Results contain only applications from the selected saved project and expose no physical dependency path. | MCP source-form and project-isolation tests plus focused LSP virtual-document cases. |
| D5: rejection table | Use private, non-exported, invalid-cased, duplicate, recovered, standard-library, transitive, ambiguous, mismatched, unresolved, incomplete, additionally qualified, and wrong-kind inputs. | Every input keeps its existing empty or failure outcome without contaminating an unrelated valid handler identity. | Table-driven shared-selection and adapter negative tests. |
| D6: retained state | Keep a live MCP cursor, then reject a dependency refresh and trigger capture and resource-admission failures. | No partial result or resource is published; the prior declaration, workspace result, resources, and cursor remain usable from their original snapshots. | Focused MCP refresh, capacity, capture-failure, and cursor tests. |

Standard-library API companion tests are not applicable because this proposal
changes a shared language-service and adapter contract, not handler runtime
behavior. Checked transcripts belong under `examples/specification/mcp/`.
Identity and rejection matrices belong in `veln-language-service`; adapters
must not add independent handler scans.

## Related Work And Design Choice

[Veln PR #1647](https://github.com/oakcask/veln/pull/1647) added the shared
workspace handler identity and LSP and MCP projection. It deliberately rejected
package-backed paths, so it establishes the reusable handler selection and
adapter boundaries rather than evidence that this proposal is implemented.

[Veln PR #1972](https://github.com/oakcask/veln/pull/1972) extended that
identity through workspace imports while preserving leaf ranges, visibility,
wrong-kind rejection, declaration inclusion, pagination, and retained-state
rules. Its package and standard-library rejection is the current boundary that
this proposal narrows for direct dependencies only.

[Veln PR #1493](https://github.com/oakcask/veln/pull/1493) admitted
direct-dependency function references from the selected project through the
same retained package and adapter model. It demonstrates that package-owned
definitions and workspace-only reference sets fit the existing navigation
result. Handler completeness, declaration identity, and exclusions remain
handler-specific and require the planned evidence above.

The current
[Editor Support](../specification/editor-support.md#lsp-navigation-formatting-and-rename)
and [MCP](../specification/mcp.md#saved-workspace-navigation) specifications
remain authoritative for implemented behavior. Reusing their shared selection
and retained-resource paths avoids an adapter-specific package-handler identity.

## Non-Goals

- Do not add standard-library or transitive-dependency handler navigation.
- Do not add dependency effect or effect-operation navigation.
- Do not add handler rename, completion, hover, signature help, or symbols.
- Do not include dependency implementation applications or cross-project
  references.
- Do not change package resolution, export validation, source-resource URIs,
  location ordering, cursors, or protocol schemas.

## Implementation And Specification Route

Extend shared handler eligibility and qualified lookup with the retained
direct-dependency package identity already used by other package symbols. Reuse
the existing handler occurrence traversal and adapter projection. Do not infer
identity from spelling or introduce a second package scan.

Add executable evidence before updating current-behavior prose. When the
evidence passes, update the handler navigation sections of `editor-support.md`
and `mcp.md`, and name the checked transcript. Remove this slice from the
agent-language-services inventory, then remove this page and its Ready catalog
entry once the current specifications and executable evidence own the
behavior.
