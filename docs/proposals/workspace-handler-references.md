---
role: proposal
update-when: Shared workspace handler selection, handle-expression syntax, LSP or MCP reference policy, or planned handler-reference evidence changes.
---

# Workspace Handler References

## Outcome And Readiness

Return references for a selected same-module workspace handler through the
shared language service and the existing LSP and MCP reference adapters. This
is a selectable slice of [Agent Language Services](agent-language-services.md).

The shared navigation index already records workspace handler declarations.
It selects the same handler identity from its declaration and the handler path
in `handle Body with handler_name(arguments)`, and definition navigation returns the
declaration. Reference collection is the remaining bounded gap: a selected
handler currently produces no reference spans. The adapters already provide
saved-source capture, coordinate conversion, declaration inclusion,
deterministic ordering, pagination, and state-preserving failures. This slice
needs no syntax, package graph, request field, or protocol result change.

## Scope

The slice covers one handler declared in a workspace module and every
parse-clean bare handler path in saved workspace sources that declare the same
module. The handler declaration is included only under each adapter's existing
declaration policy.

Imported handlers, package handlers, handler context or operation-clause
bindings, effects, effect operations, rename, and syntax-recovered paths are
excluded. Those surfaces require separate visibility, identity, recovery, or
edit contracts. This slice does not
reinterpret an unresolved or excluded occurrence as a same-spelled local
handler.

## Proposed Contract

For a selected parse-clean workspace handler, the shared result contains every
complete same-module `with handler_name(arguments)` occurrence that resolves to the
selected declaration and no occurrence owned by another identity. Selecting
the declaration or any included occurrence produces the same handler identity
and reference set.

LSP converts the shared set to zero-based UTF-16 locations. Its existing
`includeDeclaration` option controls whether the workspace declaration is
added. MCP converts the same set to one-based Unicode-scalar locations and
applies its existing `include_declaration`, ordering, page-size, and cursor
rules. Neither adapter adds a package location for this workspace-only slice.

## Acceptance Model

| Input or boundary | Observable result | Planned evidence |
| --- | --- | --- |
| Select a handler declaration or any complete `with handler_name(arguments)` occurrence in one module. | Every selection returns the same declaration identity and the complete set of handler references across that module's saved sources. Each range covers only the handler-name token. | Table-driven shared-language-service cases with independently enumerated ranges for declaration and reference selections in a multi-source module. |
| Select the handler and exclude its declaration through the adapter request. | Return only the shared reference set. | Paired LSP `includeDeclaration: false` and MCP `include_declaration: false` cases. |
| Select the handler and request its declaration. | Add the one workspace declaration without duplicating a selected occurrence. Preserve each adapter's current coordinate and ordering policy. | Paired exact-location adapter cases over LF, CRLF, and non-BMP text, plus an MCP page boundary that separates the declaration and references. |
| Another module declares a same-spelled handler, or the spelling belongs to an effect, operation, type, constructor, function, binding, field, comment, or string. | Exclude those occurrences and preserve the selected handler identity. | Shared identity-collision matrix and exact adapter results. |
| The module has duplicate handler declarations, or a declaration is malformed or syntax-recovered. | Do not choose an arbitrary declaration and return no handler reference set for the ambiguous or recovered identity. Unrelated valid handlers remain selectable. | Shared duplicate and declaration-recovery cases. |
| A handler path is imported, package-backed, qualified, unresolved, ambiguous, or incomplete, or its `handle` expression is syntax-recovered. | Preserve the current definition/reference empty or unsupported result. Do not fall back to a same-spelled workspace handler. | Negative shared and adapter matrix covering each excluded origin and recovery boundary. |
| A saved capture, position, path, MCP continuation, or retained-resource operation fails. | Preserve the existing adapter failure and state. Return no partial handler-reference result and do not consume or create unrelated state. | Existing adapter transition harness extended with one handler selection before and after the failure. |

## Verification And Completion

Implement reference collection once in `veln-language-service`. Use shared
table-driven cases as the authority for identity and source spans. Adapter
tests must verify coordinate conversion, declaration policy, MCP pagination,
and failure preservation without duplicating the lookup algorithm as their
expected-value source.

Completion requires every acceptance row to pass. Update the navigation
behavior and limits in `editor-support.md` and `mcp.md`, then remove this page
and its Ready catalog entry. Keep imported handlers, effect-operation
references, recovery, rename, broader cross-adapter conformance, and client
packaging in the umbrella until each has its own complete contract.
