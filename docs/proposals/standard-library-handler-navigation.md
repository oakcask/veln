---
role: proposal
update-when: The planned standard-library handler selection, package identity, reference scope, adapter projection, exclusions, or acceptance evidence changes.
---

# Standard-Library Handler Navigation

Navigate a public handler from the retained embedded standard library through
complete qualified applications in the selected workspace project. The shared
language service already navigates standard-library functions, types, aliases,
constructors, and schemas, and it navigates handlers owned by workspaces and
retained direct dependencies. This proposal joins those existing boundaries
for one bounded standard-library symbol class.

This proposal is ready for implementation. It changes shared navigation and
its existing LSP and MCP projections. It does not change handler syntax,
standard-library capture, effect execution, package resolution, or either
protocol.

## Scope And Identity

An eligible handler has one public, valid-cased, unrecovered declaration in an
exported module of the retained embedded `std` snapshot. A saved workspace
source imports that module from `std` with one parse-clean import. A complete
`handle Body with module::handler(arguments)` application through that import
selects the standard-library package revision, module, and handler declaration.

The handler-name leaf is the selected range. The module qualifier and import
declaration retain their existing identities. Every eligible application in
the selected project that resolves to the same retained declaration joins one
reference set. Equal spelling in a workspace, dependency package, another
standard-library module, or another symbol class does not join that identity.

The declaration uses its immutable `veln-pkg:` location. Workspace
applications use their ordinary workspace locations. Standard-library source
bodies do not contribute application references; the selected saved workspace
project is the reference universe.

## Observable Contract

Selecting an eligible application returns the retained standard-library
handler declaration as definition and the selected project's deterministic
workspace application locations as references.

LSP and MCP project the shared result through their existing coordinate,
ordering, pagination, stable-snapshot, and failure contracts. LSP references
contain only workspace applications, including when `includeDeclaration` is
true. A project-scoped MCP request with `include_declaration: true` adds the
canonical package declaration once before sorting and pagination; false omits
it. A single-file or anonymous MCP request does not add the package
declaration. Neither adapter converts the declaration to a workspace `file:`
location. Handler rename remains unsupported and produces no edits.

The following selection decisions apply:

| Input | Result |
| --- | --- |
| Public unrecovered handler in an exported retained `std` module, reached through one exact import | Shared standard-library handler definition and selected-project reference set. |
| Same handler reached through its full module path or unique import alias | The same identity and locations. An exact full path wins over a colliding alias. |
| Private, non-exported, invalid-cased, duplicate, or recovered standard-library handler | Unsupported selection: definition is `null`, references are empty, and no cursor is created. |
| Handler from a workspace, direct dependency, transitive dependency, test-only standard-library source, or another retained standard-library revision | Its own existing result or an unsupported selection; never the selected standard-library identity. |
| Ambiguous, duplicate, recovered, mismatched-package, unresolved, incomplete, additionally qualified, or wrong-kind application | Unsupported selection; no inferred handler identity. |

Repeated parse-clean imports of the same `std` module are ambiguous even when
they use different aliases. A recovered import does not poison a separate
parse-clean route. An unsupported handler leaf does not fall back to a
function, module value, type, schema, or other symbol at the same path.
Comments, strings, standard-library implementation applications, and
applications in another selected project do not enter the result. An unrelated
parse error does not remove a complete parse-clean eligible application.

MCP capture, resource admission, lookup, position, and cursor failures publish
no partial locations and preserve the accepted workspace, retained standard
library, published resources, prior results, and existing cursors. A rejected
workspace refresh cannot replace the retained declaration or attach new
applications to its identity. LSP continues to read its accepted project
snapshot and retained virtual source catalog.

## Acceptance And Verification

All evidence in this section is planned. Existing standard-library symbol and
direct-dependency handler cases establish reusable boundaries; they do not
prove the proposed standard-library handler behavior.

| Case | Input | Required observation | Planned evidence |
| --- | --- | --- | --- |
| S1: shared selection | Select `observe::noop` from several complete qualified applications in one saved project. | Every selection returns the same canonical standard-library definition and workspace application set; ranges cover only handler names. | Focused language-service identity tests plus LSP definition and references tests. |
| S2: adapter agreement | Request definition and declaration-inclusive and declaration-excluding references through LSP and MCP from the same saved snapshot without an LSP overlay. | After coordinate conversion, both adapters project the same workspace locations. LSP omits the package declaration; project-scoped MCP includes it exactly once only when requested, single-file MCP omits it, and both retain their ordering and pagination contracts. | Checked MCP transcript and saved-navigation cross-adapter coverage. |
| S3: owner and revision isolation | Provide equal module and handler names in a dependency and workspace module, then repeat the selection across sequential snapshots with different retained standard-library revisions. | Each selection retains its exact `std` owner and revision. No location is rewritten to another owner or stale revision. | Shared-language-service collision tests plus adapter snapshot tests. |
| S4: source and project boundaries | Place equal applications in standard-library bodies, a different workspace project, comments, and strings, then select each workspace project independently. | Each result contains only applications from its selected saved project and exposes no standard-library build path. | MCP project-isolation tests plus focused LSP virtual-document cases. |
| S5: rejection table | Use private, non-exported, invalid-cased, duplicate, recovered, ambiguous-import, duplicate-import, transitive, mismatched, unresolved, incomplete, additionally qualified, test-only, and wrong-kind inputs. | Every input keeps its existing empty or separate result without contaminating an unrelated valid handler identity. | Table-driven shared-selection and adapter negative tests. |
| S6: retained state | Keep a live MCP cursor, then reject a workspace capture and trigger resource-admission, position, and cursor failures. | Capture and admission failures publish no partial result or resource. Position and cursor failures preserve the prior declaration, workspace result, resources, and existing cursors at their original snapshots. | Focused MCP refresh, capacity, capture-failure, position, and cursor tests. |

Standard-library API companion tests are not applicable because this proposal
changes a shared language-service and adapter contract, not handler runtime
behavior. Checked transcripts belong under `examples/specification/mcp/`.
Identity and rejection matrices belong in `veln-language-service`; adapters
must not add independent handler scans.

## Related Work And Design Choice

[Veln PR #1981](https://github.com/oakcask/veln/pull/1981) added the shared
retained-package handler identity and LSP and MCP projection for direct
dependencies. Its implementation deliberately rejects standard-library
owners, so it establishes the reusable handler traversal, package declaration,
adapter, and failure-state boundaries rather than evidence that this proposal
is implemented.

[Veln PR #1251](https://github.com/oakcask/veln/pull/1251) added retained
embedded-standard-library definitions and readable immutable virtual sources.
It establishes that the standard-library package revision and canonical
`veln-pkg:` locations are already navigation identities; handler eligibility
and application membership remain handler-specific work.

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

Extend shared handler eligibility and qualified lookup with the retained
standard-library package identity already used by other package symbols. Reuse
the existing handler occurrence traversal and adapter projection. Do not infer
identity from spelling or introduce a second standard-library scan.

Add executable evidence before updating current-behavior prose. When the
evidence passes, update the handler navigation sections of `editor-support.md`
and `mcp.md`, and name the checked transcript. Remove this slice from the
agent-language-services inventory, then remove this page and its Ready catalog
entry once the current specifications and executable evidence own the
behavior.
