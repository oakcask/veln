---
role: proposal
update-when: Workspace import resolution, imported effect navigation, effect-operation identity, LSP or MCP reference policy, or planned imported-operation evidence changes.
---

# Workspace Imported-Effect-Operation References

## Outcome And Readiness

Link an operation of a public effect from an explicitly imported workspace
module to the qualified operation-name leaves that invoke it. Return that
identity through the shared language-service navigation result and the
existing LSP and MCP definition and reference adapters. This is a selectable
slice of [Agent Language Services](agent-language-services.md).

The checker already resolves operations of public workspace effects through a
written `use`. The shared navigation service already links same-module effect
operation declarations, `perform` leaves, and eligible handler-clause headings.
It also resolves a parse-clean workspace import to the public owning effect and
retains the qualified `perform` shape, but it deliberately excludes the
adjacent imported operation leaf. Connecting that leaf to the existing
operation identity is the remaining bounded gap.

The adapters already provide saved-project capture, coordinate conversion,
declaration inclusion, deterministic ordering, MCP pagination, and
state-preserving failures. This slice needs no source syntax, package graph,
request field, protocol result, handler-import, or rename change.

## Scope

The slice covers an operation declared by one valid-cased public effect in a
workspace module selected by one parse-clean explicit workspace import. The
import can be selected by its full written module path or by its unique
implicit leaf alias under the current import-resolution rules. The imported
occurrence is the operation-name leaf in a complete qualified
`perform module::Effect::operation(arguments)` expression. The module
qualifier, effect-name leaf, and operation-name leaf keep their existing
separate identities.

Same-module operation declarations, `perform` leaves, and eligible
handler-clause headings remain part of the selected operation's reference set.
Private effects, direct and transitive dependencies, the standard library,
imported handler-clause headings, generic effect parameters, and rename are
outside this slice. This proposal does not make an operation eligible merely
because its spelling matches a workspace declaration.

## Proposed Contract

For an eligible import, selecting the effect-operation declaration, an eligible
same-module reference, or a qualified imported operation-name leaf produces
one identity containing the declaring workspace module, owning effect, and
operation name. Definition returns the effect-operation declaration.
References contain the existing eligible same-module references and every
qualified operation leaf whose import resolves to that same module and public
owning effect. Each imported range covers only the operation-name leaf.

LSP converts the shared result to retained workspace `file:` URIs and
zero-based UTF-16 locations. MCP converts the same result to canonical saved
workspace `file:` URIs and one-based Unicode-scalar locations. Their existing
declaration-inclusion, sorting, pagination, capture, and failure contracts
remain unchanged. Neither adapter returns a package location for this
workspace-only slice.

## Acceptance Model

| Input or boundary | Observable result | Planned evidence |
| --- | --- | --- |
| A consumer explicitly imports a workspace module that declares one public effect and performs one of its operations. | Selecting the declaration, a same-module operation reference, or the imported operation leaf returns the same workspace effect-operation definition and complete same-module and imported reference set. Every imported range covers only the operation-name leaf. | Table-driven shared-language-service cases with independently enumerated locations across declaring and consuming sources and every selection form. |
| A nested module path is used through its full written path or one unique implicit leaf alias. | Both accepted spellings select the same declaring module, owning effect, and operation. An exact full-path import takes precedence over a colliding implicit leaf alias under the current import rules. | Shared import-resolution matrix for full-path, unique-leaf, and full-path-versus-leaf precedence. |
| The declaring module also contains a parse-clean handler for the owning effect and same-module `perform` expressions. | The existing eligible handler-clause headings and same-module operation leaves remain in the imported operation's reference set. A qualified handler target in the importing module does not add its clause headings. | Shared mixed-reference cases covering declaration, same-module `perform`, handler heading, and imported `perform` locations. |
| Either adapter requests references without the declaration. | Return the same eligible workspace references without the operation declaration, import path, module qualifier, or effect-name leaf. | Paired LSP `includeDeclaration: false` and MCP `include_declaration: false` exact-location cases. |
| Either adapter requests the declaration, and the source contains CRLF text or a non-BMP scalar before an imported operation leaf. | Add the declaration once and preserve each adapter's current coordinate and ordering policy. MCP pagination across the declaration and operation leaves preserves the complete set. | Paired LSP and MCP coordinate cases plus an MCP continuation case. |
| Two imports expose equal effect and operation spellings, an import is duplicate or recovered, or its full path and implicit alias do not resolve uniquely. | Return no arbitrary imported-operation identity. Preserve navigation for unrelated valid same-module and imported operations. | Shared collision, duplicate, ambiguity, and recovery matrix with exact empty or unaffected results. |
| The owning effect is private, duplicate, invalid-cased, recovered, or unresolved; the operation is duplicate, invalid-cased, recovered, or unknown; or either declaration is reached through a dependency or the standard library. | Preserve the current empty or unsupported imported-operation result. Do not fall back to a same-spelled workspace operation. | Shared and adapter visibility, casing, declaration recovery, package-origin, standard-library, and unresolved cases. |
| A qualified `perform` is missing an effect name, operation name, opening `(`, or closing `)`, has an additionally qualified operation path, or has an argument-list recovery diagnostic. | Exclude the operation leaf without changing the adjacent effect leaf's independently eligible imported-effect result. Selecting the import path, module qualifier, or effect leaf does not reinterpret that token as the operation. | Token-boundary and malformed-shape cases aligned with parser recovery and the current effect-operation rules. |
| A saved capture, position, path, MCP continuation, or retained-resource operation fails. | Preserve the existing adapter failure and state. Return no partial imported-operation result and do not create or consume unrelated state. | Existing adapter transition harness extended with one imported-operation selection before and after the failure. |

## Verification And Completion

Implement imported effect-operation resolution once in
`veln-language-service`. Reuse the workspace import and owning-effect identity
retained by the effective project snapshot; do not infer a target from
qualifier or operation spelling alone. Shared table-driven cases are the
authority for import selection, operation identity, visibility, recovery, and
source spans. Adapter tests verify coordinate conversion, declaration policy,
MCP pagination, and failure preservation without duplicating import lookup as
their expected-value source.

Completion requires every acceptance row to pass. Update the
effect-operation-navigation behavior and limits in `editor-support.md` and
`mcp.md`, then remove this page and its Ready catalog entry. Keep imported
handlers, package and transitive-dependency navigation, broader symbol
coverage, the conformance gate, and client packaging in the umbrella until
each has its own complete contract.
