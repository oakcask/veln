---
role: proposal
update-when: Workspace import resolution, effect visibility, shared effect selection, LSP or MCP reference policy, or planned imported-effect evidence changes.
---

# Workspace Imported-Effect References

## Outcome And Readiness

Link a public effect from an explicitly imported workspace module to the
qualified effect-name leaves that use it. Return that identity through the
shared language-service navigation result and the existing LSP and MCP
definition and reference adapters. This is a selectable slice of
[Agent Language Services](agent-language-services.md).

The checker already resolves a public workspace effect through a written
`use` and validates qualified effect rows, handler targets, and `perform`
qualifiers. The shared navigation index already retains workspace imports,
effect declarations, and the qualified occurrence shapes. It deliberately
limits effect selection to bare same-module names. Connecting an eligible
qualified effect-name leaf to the existing workspace effect identity is the
remaining bounded gap.

The adapters already provide saved-project capture, coordinate conversion,
declaration inclusion, deterministic ordering, MCP pagination, and
state-preserving failures. This slice needs no source syntax, package graph,
request field, protocol result, or rename change.

## Scope

The slice covers a valid-cased public effect in a workspace module selected by
one parse-clean explicit workspace import. The import can be selected by its
full written module path or by its unique implicit leaf alias under the current
import-resolution rules. Eligible occurrences are the effect-name leaf in a
qualified function, test, handler, or function-type effect row, a qualified
handler `handles` target, and the effect qualifier in a complete qualified
`perform` expression. The import path and module qualifier keep their module
identity. An effect-operation name keeps its separate identity.

Private effects, including exact-companion access, are excluded. Direct and
transitive dependencies, the standard library, imported effect operations and
handlers, generic effect parameters, and rename are outside this slice. This
proposal does not make a qualified effect eligible merely because its spelling
matches a workspace declaration.

## Proposed Contract

For an eligible import, selecting the public effect declaration or any
qualified effect-name leaf produces one identity containing the declaring
workspace module and effect name. Definition returns the effect declaration.
References contain all eligible bare same-module occurrences and qualified
occurrences whose import resolves to that same module and declaration. Each
qualified reference range covers only the effect-name leaf, not the import
alias or the complete path.

LSP converts the shared result to retained workspace `file:` URIs and
zero-based UTF-16 locations. MCP converts the same result to canonical saved
workspace `file:` URIs and one-based Unicode-scalar locations. Their existing
declaration-inclusion, sorting, pagination, capture, and failure contracts
remain unchanged. Neither adapter returns a package location for this
workspace-only slice.

## Acceptance Model

| Input or boundary | Observable result | Planned evidence |
| --- | --- | --- |
| A consumer explicitly imports a workspace module that declares one public effect, then uses the qualified effect in an effect row, a function-type effect row, a handler target, and a complete `perform`. | Selecting the declaration or any effect-name leaf returns the same workspace effect definition and complete bare and qualified reference set. Every qualified range covers only the effect-name leaf. | Table-driven shared-language-service cases with independently enumerated locations across declaring and consuming sources and every selection form. |
| A nested module path is used through its full written path or one unique implicit leaf alias. | Both accepted spellings select the same declaring module and effect. An exact full-path import takes precedence over a colliding implicit leaf alias under the current import rules. | Shared import-resolution matrix for full-path, unique-leaf, and full-path-versus-leaf precedence. |
| Either adapter requests references without the declaration. | Return the same eligible workspace references without the effect declaration or import path. | Paired LSP `includeDeclaration: false` and MCP `include_declaration: false` exact-location cases. |
| Either adapter requests the declaration, and the source contains CRLF text or a non-BMP scalar before a reference. | Add the declaration once and preserve each adapter's current coordinate and ordering policy. MCP pagination across a declaration and qualified references preserves the complete set. | Paired LSP and MCP coordinate cases plus an MCP continuation case. |
| Two imports expose equal effect spellings, an import is duplicate or recovered, or its full path and implicit alias do not resolve uniquely. | Return no arbitrary imported-effect identity. Preserve navigation for unrelated valid same-module and imported effects. | Shared collision, duplicate, ambiguity, and recovery matrix with exact empty or unaffected results. |
| The effect is private, duplicate, invalid-cased, recovered, unresolved, or reached through a dependency or the standard library. | Preserve the current empty or unsupported imported-effect result. Do not fall back to a same-spelled workspace effect. | Shared and adapter visibility, casing, recovery, package-origin, standard-library, and unresolved cases. |
| A qualified `perform` is missing its operation path, opening `(`, or closing `)`, or the selected token is the module qualifier, import path, or operation leaf. | Do not reinterpret the selected token as the imported effect. Exclude an incomplete effect qualifier, but keep a structurally complete effect qualifier eligible when only the operation is unknown or its argument list requires recovery. | Token-boundary and malformed-shape cases aligned with parser recovery and existing effect-reference rules. |
| A saved capture, position, path, MCP continuation, or retained-resource operation fails. | Preserve the existing adapter failure and state. Return no partial imported-effect result and do not create or consume unrelated state. | Existing adapter transition harness extended with one imported-effect selection before and after the failure. |

## Verification And Completion

Implement imported-effect resolution once in `veln-language-service`. Reuse
the workspace import identity retained by the effective project snapshot; do
not infer a target from qualifier spelling alone. Shared table-driven cases
are the authority for import selection, effect identity, visibility, and
source spans. Adapter tests verify coordinate conversion, declaration policy,
MCP pagination, and failure preservation without duplicating import lookup as
their expected-value source.

Completion requires every acceptance row to pass. Update the effect-navigation
behavior and limits in `editor-support.md` and `mcp.md`, then remove this page
and its Ready catalog entry. Keep imported effect operations and handlers,
package and transitive-dependency navigation, broader symbol coverage, the
conformance gate, and client packaging in the umbrella until each has its own
complete contract.
