---
role: proposal
update-when: Shared workspace effect-operation selection, handler operation-clause syntax, LSP or MCP reference policy, or planned handler operation-clause evidence changes.
---

# Workspace Handler Operation-Clause References

## Outcome And Readiness

Link a same-module workspace handler operation-clause heading to the effect
operation that the handler implements. Return that heading through the shared
language-service reference result and the existing LSP and MCP reference
adapters. This is a selectable slice of
[Agent Language Services](agent-language-services.md).

The shared navigation index already records workspace effect and operation
declarations. It selects an effect from a handler's bare `handles` target and
selects the same operation identity from its declaration and complete
`perform Effect::operation(arguments)` leaves. The syntax tree also retains
each handler operation-clause heading and semantic analysis checks it against
the handled effect. Navigation currently recognizes the heading position but
deliberately returns no symbol for it. Connecting that retained heading to the
existing same-module operation identity is the remaining bounded gap.

The adapters already provide saved-source capture, coordinate conversion,
declaration inclusion, deterministic ordering, pagination, and
state-preserving failures. This slice needs no syntax, package graph, request
field, or protocol result change.

## Scope

The slice covers a parse-clean workspace handler whose bare `handles` target
resolves to one effect declaration in the same module. Each structurally
complete operation-clause heading that resolves to one operation of that
effect joins the existing reference set for that operation. The operation
identity contains the module, owning effect, and operation name. The effect
qualifier and handler declaration keep their separate identities.

Imported, qualified, direct-dependency, transitive-dependency, and
standard-library handled effects are excluded. Duplicate or recovered effects,
operations, handlers, and clause headings are excluded. Handler clause
parameters, clause bodies, rename, and casing recovery remain governed by
their existing contracts. This slice does not match a clause heading by
spelling when its handler's `handles` target cannot select the owning effect.

## Proposed Contract

For an eligible effect operation, the shared reference result contains its
complete same-module `perform` leaves and every eligible operation-clause
heading in a handler for that effect. Selecting the operation declaration, a
complete `perform` leaf, or an eligible clause heading produces the same
operation identity, definition, and reference set. Each clause reference range
covers only the operation-name token.

LSP converts the shared set to zero-based UTF-16 locations. Its existing
`includeDeclaration` option controls whether the effect-operation declaration
is added. MCP converts the same set to one-based Unicode-scalar locations and
applies its existing `include_declaration`, ordering, page-size, and cursor
rules. Neither adapter adds a handler declaration or package location for this
workspace-only slice.

## Acceptance Model

| Input or boundary | Observable result | Planned evidence |
| --- | --- | --- |
| Select an operation declaration, complete `perform` leaf, or matching clause heading where the handler and effect are declared in one workspace module. | Every selection returns the same module, effect, and operation identity, definition, and complete set of `perform` and clause-heading references across that module's saved sources. Each range covers only the operation-name token. | Table-driven shared-language-service cases with independently enumerated ranges for all three selection forms in a multi-source module. |
| Select the operation through either adapter without requesting its declaration. | Return all eligible `perform` and clause-heading references without the effect-operation declaration or handler declaration. | Paired LSP `includeDeclaration: false` and MCP `include_declaration: false` exact-location cases. |
| Select the operation and request its declaration. | Add the one effect-operation declaration without duplicating a reference. Preserve each adapter's current coordinate and ordering policy. | Paired adapter cases over LF, CRLF, and non-BMP text, plus an MCP page boundary that separates the declaration, a `perform` leaf, and a clause heading. |
| Another effect or module has the same operation spelling, or a different handler handles that other effect. | Include a clause heading only in the reference set for the effect named by its enclosing handler's resolved `handles` target. | Shared identity-collision matrix with two effects, two handlers, and repeated operation spellings across modules. |
| A handler has duplicate headings, a heading names no operation of the handled effect, or the effect, operation, handler, `handles` target, or heading is duplicate, malformed, ambiguous, or syntax-recovered. | Do not choose an arbitrary operation or add a spelling-only heading. Preserve navigation for unrelated valid operations and handlers. | Shared negative and recovery cases aligned with semantic handler diagnostics, with exact empty or unaffected reference results. |
| The handled effect is imported, qualified, package-backed, or otherwise outside the same-module workspace boundary. | Preserve the current empty or unsupported navigation result for the clause heading. Do not fall back to a same-spelled workspace effect or operation. | Shared and adapter origin-boundary cases for imported, qualified, direct-package, standard-library, unresolved, and ambiguous targets. |
| A saved capture, position, path, MCP continuation, or retained-resource operation fails. | Preserve the existing adapter failure and state. Return no partial clause-heading result and do not consume or create unrelated state. | Existing adapter transition harness extended with one clause-heading selection before and after the failure. |

## Verification And Completion

Implement handler-heading resolution once in `veln-language-service`. Use
shared table-driven cases as the authority for the owning effect, operation
identity, and source spans. Adapter tests must verify coordinate conversion,
declaration policy, MCP pagination, and failure preservation without
duplicating the lookup algorithm as their expected-value source.

Completion requires every acceptance row to pass. Update the navigation
behavior and limits in `editor-support.md` and `mcp.md`, then remove this page
and its Ready catalog entry. Keep imported and package-backed effects,
operations, and handlers, recovery, rename, broader cross-adapter conformance,
and client packaging in the umbrella until each has its own complete contract.
