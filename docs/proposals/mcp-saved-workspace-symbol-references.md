---
role: proposal
update-when: Shared workspace type, constructor, or lexical-binding references, the MCP references tool contract, saved navigation capture, or planned reference evidence changes.
---

# MCP Saved Workspace Symbol References

## Summary

Expose the shared language-service reference sets for valid workspace types,
constructors, and lexical bindings through the existing MCP `references` tool.
The tool already returns saved workspace function references. This slice closes
the remaining valid, reference-bearing workspace symbol classes without adding
pagination, dependency references, recovery selection, or a new wire shape.

## Scope

| Included | Excluded |
| --- | --- |
| Workspace type declarations and resolved type-role occurrences. | Schema, effect, handler, and effect-operation references. |
| Workspace constructor declarations, calls, and patterns. | Package, dependency, standard-library, and virtual-source references. |
| Function parameters, result bindings, local `let` and pattern bindings, satisfy candidate bindings, handler context parameters, and handler operation-clause parameters. | Invalid-declaration or invalid-binding recovery records. |
| Existing project and anonymous single-file scopes, saved capture, ordering, and failure behavior. | Declaration inclusion controls, pagination, cursors, overlays, prepare-rename, rename edits, and client plugins. |

The shared language service remains authoritative for symbol selection,
identity, visibility, shadowing, and reference collection. This proposal only
widens the MCP adapter's accepted valid workspace symbol kinds. It does not
change the checked `references` input or result schemas.

## Selection Contract

The server uses the same immutable saved-project capture and source-position
contract as the current `references` implementation. A result is eligible only
when the shared navigation result selects a non-recovery workspace symbol in
one of these classes:

- type;
- constructor;
- value binding;
- handler context parameter; or
- handler operation-clause parameter.

The selected declaration and every returned reference must belong to the
captured workspace snapshot. A type or constructor selected through a visible
imported workspace module keeps the declaration identity chosen by shared name
resolution. A lexical binding keeps its exact lexical identity. Equal spellings
in another namespace or scope do not become references.

The existing function result remains unchanged. A valid position that selects
a casing-neutral symbol, a recovery record, a package-backed symbol, or no
symbol succeeds with an empty `references` array. This slice does not reinterpret
an unsupported selection as another symbol class.

## Result And State Contract

Each returned location uses the existing canonical workspace `file:` URI and
one-based Unicode-scalar half-open range. Locations contain reference sites
only; the selected declaration is not added to the result. The adapter
preserves the shared deterministic order and removes no identity-relevant
distinction through URI conversion.

A source owned by a selected manifest project returns project scope with
`project_wide: true`. An accepted source outside the selected owned-source set
returns single-file scope with `project_wide: false`; its result cannot include
another saved source. The selection generation and existing scope fields remain
unchanged.

Stable-capture exhaustion returns `snapshot_changed` with no success-only
reference or scope members. Dependency resource admission still occurs under
the existing operation-atomic contract even though this slice returns only
workspace locations. A capacity failure returns `resource_capacity`, publishes
no reference result or new resources, and preserves earlier retained resources.
Invalid paths and positions retain their current domain failures. No failure or
unsupported-symbol success changes workspace selection state.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select a workspace type at its declaration, a bare resolved type occurrence, or a qualified type occurrence. | Each selection returns the same sorted reference sites for that exact type identity, excluding the declaration and same-spelled declarations from other visible modules. | Table-driven MCP type cases with local, imported, qualified, and ambiguous spellings. |
| Select a workspace constructor at its declaration, call, nullary expression use, or pattern use. | Each selection returns only calls and patterns for that constructor identity, excluding the declaration, another type's same-spelled constructor, and equal-spelled effect operations. | Table-driven constructor cases covering qualified, bare, nullary, pattern, alias-visible, and collision boundaries. |
| Select a function parameter, result binding, local `let` or pattern binding, or satisfy candidate binding. | The result contains only uses bound to that declaration inside its lexical scope; shadowed and pre-declaration occurrences are excluded. | Lexical-binding matrix with nested scopes, initializer boundaries, callable uses, and same-spelled fields. |
| Select a handler context parameter or handler operation-clause parameter. | The result follows the selected parameter through its valid handler or clause scope and excludes occurrences captured by an inner binding. | Handler-binding matrix with context, clause, callable, and shadowing cases. |
| Select the same supported symbol from a source outside the selected project's owned-source set. | The result is limited to the accepted source and reports single-file scope with `project_wide: false`. | Anonymous and descendant-package isolation cases. |
| Select a workspace function already supported by MCP. | Its current reference locations and scope metadata remain unchanged. | Existing function-reference executable case plus mixed-symbol regression coverage. |
| Select a schema, effect, handler, effect operation, recovery record, package-backed symbol, unsupported occurrence, or no symbol. | The request succeeds with an empty `references` array and does not reinterpret the selection. | Unsupported-class and source-origin decision table. |
| Use LF, CRLF, non-ASCII text, token-end positions, or an unaddressable positive coordinate. | Valid positions select the same semantic identities under Unicode-scalar coordinates; token ends do not select, and unaddressable positions return `invalid_position`. | Coordinate matrix shared with saved definition and function-reference cases. |
| Change captured source identity, bytes, or path membership during reference capture. | Capture retries under the existing bound, then returns `snapshot_changed` without partial locations. | Injected capture-change cases for project and single-file scopes. |
| Reach dependency resource capacity during a reference operation. | The operation returns `resource_capacity`, admits no new resource, returns no partial reference set, and preserves earlier resources. | Capacity-boundary state-preservation case. |

## Completion

This proposal is complete when every acceptance row passes and the MCP
specification and executable MCP examples state the implemented workspace
symbol-reference boundary. Move completion history under
`../reference/implemented-proposals/`, remove this page from the Ready catalog,
and leave the umbrella proposal unselectable until another finite slice is
extracted.
