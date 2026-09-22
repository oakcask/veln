---
role: proposal
update-when: Shared workspace effect selection, effect reference syntax, LSP or MCP reference policy, or planned effect-reference evidence changes.
---

# Workspace Effect References

## Outcome And Readiness

Return references for a selected same-module workspace effect through the
shared language service and the existing LSP and MCP reference adapters. This
is a selectable slice of [Agent Language Services](agent-language-services.md).

The shared navigation index already records workspace effect declarations and
selects the same effect identity from its declaration, an effect row, a
handler's `handles` target, and the qualifier of a `perform` expression. It
also returns the declaration through definition. Reference collection is the
remaining bounded gap: the selected effect currently produces no reference
spans. The existing adapters already provide saved-source capture, coordinate
conversion, declaration inclusion, deterministic ordering, pagination, and
state-preserving failures. No syntax, package graph, new request field, or new
protocol result is required.

## Scope

The slice covers one effect declared in a workspace module and references in
all saved workspace sources that declare that same module. It preserves the
selected effect identity across:

- effect rows on functions, tests, handlers, and function types;
- a handler's `handles` target; and
- the effect qualifier in `perform Effect::operation(...)`.

The effect declaration is included only under each adapter's existing
declaration policy. The `perform` operation leaf remains a distinct effect
operation identity and is not an effect reference.

Imported workspace effects, direct dependencies, the standard library,
transitive packages, generic effect parameters, handlers, effect operations,
rename, casing recovery, and syntax-recovered paths are excluded. Those
surfaces require separate visibility, identity, recovery, or edit contracts.
This slice does not reinterpret an unresolved or excluded occurrence as the
same-module effect.

## Proposed Contract

For a selected parse-clean workspace effect, the shared result contains every
same-module occurrence in the included source forms and no occurrence owned by
another effect identity. Selecting the declaration or any included occurrence
produces the same effect identity and reference set.

LSP converts that shared set to zero-based UTF-16 locations. Its existing
`includeDeclaration` option controls whether the workspace declaration is
added. MCP converts the same set to one-based Unicode-scalar locations and
applies its existing `include_declaration`, sorting, page size, and cursor
rules. Neither adapter adds a package location for this workspace-only slice.

## Acceptance Model

| Input or boundary | Observable result | Planned evidence |
| --- | --- | --- |
| Select an effect declaration, an effect-row occurrence, a `handles` target, or a `perform` qualifier in one module. | Every selection returns the same declaration identity and the same complete set of effect-row, `handles`, and `perform`-qualifier references across that module's saved sources. | Table-driven shared-language-service cases with independently enumerated source ranges for every selection form and a multi-source module. |
| Select an effect and exclude its declaration through the adapter request. | Return only the shared reference set. | Paired LSP `includeDeclaration: false` and MCP `include_declaration: false` cases. |
| Select the effect and request its declaration. | Add the one workspace declaration, without duplicating a declaration already selected as input. Preserve each adapter's current location encoding and ordering policy. | Paired exact-location adapter cases over LF, CRLF, and non-BMP text, plus an MCP page boundary that places the declaration and references on different pages. |
| A same-spelled effect exists in another module, or the same spelling appears as a handler, operation, type, function, field, comment, or string. | Exclude those occurrences and preserve the selected effect's identity. | Shared collision matrix and exact adapter results. |
| Select the operation leaf in `perform Effect::operation(...)`. | Preserve the current effect-operation selection and do not return the qualifier's effect references. Selecting the qualifier still returns the effect references. | Adjacent-token selection cases for qualifier and operation positions. |
| Select an imported, package, generic-parameter, invalid-cased, ambiguous, unresolved, or syntax-recovered effect-shaped occurrence. | Preserve the current definition/reference empty or unsupported result. Do not fall back to a same-spelled workspace effect. | Negative shared and adapter matrix covering every excluded origin and recovery boundary. |
| A saved capture, position, path, MCP continuation, or retained-resource operation fails. | Preserve the existing adapter failure and state. Return no partial effect-reference result and do not consume or create unrelated state. | Existing adapter transition harness extended with one effect selection before and after the failure. |

## Verification And Completion

Implement reference collection once in `veln-language-service`. Use shared
table-driven cases as the authority for identity and source spans. Adapter
tests must verify coordinate conversion, declaration policy, MCP pagination,
and failure preservation without duplicating the lookup algorithm as their
expected-value source.

Completion requires every acceptance row to pass. Update the navigation
behavior and limits in `editor-support.md` and `mcp.md`, then remove this page
and its Ready catalog entry. Keep imported and package effect navigation,
handler and effect-operation references, recovery, rename, broader
cross-adapter conformance, and client packaging in the umbrella until each has
its own complete contract.
