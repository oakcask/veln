---
role: proposal
update-when: Shared workspace effect-operation selection, perform-expression syntax, LSP or MCP reference policy, or planned effect-operation-reference evidence changes.
---

# Workspace Effect-Operation References

## Outcome And Readiness

Return references for a selected same-module workspace effect operation
through the shared language service and the existing LSP and MCP reference
adapters. This is a selectable slice of
[Agent Language Services](agent-language-services.md).

The shared navigation index already records workspace effect-operation
declarations. It selects the same operation identity from its declaration and
the operation leaf in a complete `perform Effect::operation(arguments)`
expression, and definition navigation returns the declaration. Reference
collection is the remaining bounded gap: a selected effect operation currently
produces no reference spans. The adapters already provide saved-source capture,
coordinate conversion, declaration inclusion, deterministic ordering,
pagination, and state-preserving failures. This slice needs no syntax, package
graph, request field, or protocol result change.

## Scope

The slice covers one operation declared by an effect in a workspace module and
every parse-clean operation leaf in complete `perform` expressions from saved
workspace sources that declare the same module. The operation identity contains
the module, owning effect, and operation name. The declaration is included only
under each adapter's existing declaration policy.

Handler operation-clause headings, imported effects, direct or transitive
package effects, the standard library, rename, casing recovery, and
syntax-recovered operation paths are excluded. Handler headings require a
separate contract that resolves each handler's `handles` target before it can
associate a heading with an effect operation. This slice does not reinterpret
an unresolved or excluded leaf as a same-spelled local operation.

## Proposed Contract

For a selected parse-clean workspace effect operation, the shared result
contains every complete same-module `perform Effect::operation(arguments)` leaf
that resolves to the selected declaration and no leaf owned by another effect
or module. Selecting the declaration or any included leaf produces the same
operation identity and reference set. Each returned range covers only the
operation-name token; the effect qualifier keeps its separate effect identity.

LSP converts the shared set to zero-based UTF-16 locations. Its existing
`includeDeclaration` option controls whether the workspace declaration is
added. MCP converts the same set to one-based Unicode-scalar locations and
applies its existing `include_declaration`, ordering, page-size, and cursor
rules. Neither adapter adds a package location for this workspace-only slice.

## Acceptance Model

| Input or boundary | Observable result | Planned evidence |
| --- | --- | --- |
| Select an operation declaration or any complete `perform Effect::operation(arguments)` leaf in one module. | Every selection returns the same module, effect, and operation identity and the complete set of matching leaves across that module's saved sources. Each range covers only the operation-name token. | Table-driven shared-language-service cases with independently enumerated ranges for declaration and leaf selections in a multi-source module. |
| Select the operation and exclude its declaration through the adapter request. | Return only the shared reference set. | Paired LSP `includeDeclaration: false` and MCP `include_declaration: false` cases. |
| Select the operation and request its declaration. | Add the one workspace declaration without duplicating a selected leaf. Preserve each adapter's current coordinate and ordering policy. | Paired exact-location adapter cases over LF, CRLF, and non-BMP text, plus an MCP page boundary that separates the declaration and references. |
| Another effect or module declares the same operation name, or the spelling belongs to an effect, handler clause, function, constructor, binding, field, comment, or string. | Exclude those occurrences and preserve the selected module, effect, and operation identity. The adjacent effect qualifier remains a separate effect reference. | Shared identity-collision matrix and exact adapter results. |
| The owning effect or operation declaration is duplicated, malformed, or syntax-recovered. | Do not choose an arbitrary declaration and return no operation reference set for the ambiguous or recovered identity. Unrelated valid operations remain selectable. | Shared duplicate and declaration-recovery cases. |
| A perform path is imported, package-backed, unresolved, ambiguous, incomplete, or syntax-recovered. | Preserve the current definition/reference empty or unsupported result. Do not fall back to a same-spelled workspace operation. | Negative shared and adapter matrix covering each excluded origin and recovery boundary. |
| A saved capture, position, path, MCP continuation, or retained-resource operation fails. | Preserve the existing adapter failure and state. Return no partial operation-reference result and do not consume or create unrelated state. | Existing adapter transition harness extended with one operation selection before and after the failure. |

## Verification And Completion

Implement reference collection once in `veln-language-service`. Use shared
table-driven cases as the authority for identity and source spans. Adapter tests
must verify coordinate conversion, declaration policy, MCP pagination, and
failure preservation without duplicating the lookup algorithm as their
expected-value source.

Completion requires every acceptance row to pass. Update the navigation
behavior and limits in `editor-support.md` and `mcp.md`, then remove this page
and its Ready catalog entry. Keep handler-clause headings, imported and package
operations, recovery, rename, broader cross-adapter conformance, and client
packaging in the umbrella until each has its own complete contract.
