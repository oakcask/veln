---
role: proposal
update-when: Shared direct-dependency function references, the MCP references tool contract, saved dependency capture, package visibility, or planned dependency-reference evidence changes.
---

# MCP Saved Dependency Function References

## Summary

Return references to visible direct-dependency functions from the selected
saved workspace project through the existing MCP `references` tool. The tool
already returns reference-only locations for supported workspace symbols, and
the `definition` tool already selects retained direct-dependency functions.
This slice connects those two implemented boundaries without adding a new wire
shape.

## Scope

| Included | Excluded |
| --- | --- |
| Public functions selected from exported modules of one retained direct dependency. | Standard-library, transitive-dependency, private, non-exported, invalid-casing, and recovery symbols. |
| Qualified calls and qualified function-value references in the inferred selected project. | References inside dependency sources or another selected project. |
| Exact dependency identity, module identity, import-alias resolution, semantic symbol identity, and shadowing boundaries. | Dependency types, constructors, schemas, public function aliases, and other package symbol classes. |
| Existing saved capture, project scope metadata, deterministic ordering, and operation-atomic resource admission. | Declaration inclusion, pagination, cursors, overlays, prepare-rename, rename edits, and client plugins. |

The shared language service remains authoritative for function selection,
visibility, import resolution, identity, and reference collection. The MCP
adapter remains authoritative for saved-project inference, location conversion,
scope metadata, and resource admission. This proposal only admits the shared
direct-dependency function result through the existing MCP boundary. It does
not change the checked `references` input or result schemas.

## Selection Contract

The server uses the same immutable saved-project capture and source-position
contract as the current `references` implementation. A result is eligible only
when all of these facts hold:

- the requested source belongs to an inferred selected manifest project;
- the selected occurrence resolves to a function in a retained direct
  dependency;
- the source writes the exact external package and module import required by
  current name resolution;
- the dependency identity matches that import;
- the function source belongs to the dependency's validated export set;
- the function is public and has no invalid source identifier casing record;
  and
- the occurrence is a qualified call target or qualified function-value
  reference selected by the shared language service.

The result contains only occurrences in the selected project's captured owned
sources that resolve to the same function identity. Equal spellings in another
dependency, another module, a workspace declaration, a lexical binding, a
field, a comment, or a string are not references. Dependency implementation
bodies and other selected projects are outside the search universe.

A valid position that selects a standard-library function, dependency function
alias, non-function package symbol, package module segment, unsupported
occurrence, or no symbol keeps the existing successful empty result. This
slice does not reinterpret an unsupported selection as a function.

## Result And State Contract

Each returned reference uses the existing canonical workspace `file:` URI and
one-based Unicode-scalar half-open range. The dependency declaration is not
included, and the result contains no `veln-pkg:` location. Locations preserve
the shared deterministic order and semantic identity through MCP conversion.

Every eligible result reports project scope for the inferred selected project
with `project_wide: true`. Anonymous single-file analysis cannot acquire a
dependency universe and continues to return an empty result for package-backed
selection. The selection generation and existing scope fields remain
unchanged.

Stable-capture exhaustion returns `snapshot_changed` without success-only
reference or scope members. Dependency resource admission remains
operation-atomic. A capacity failure returns `resource_capacity`, publishes no
reference result or new resources, and preserves earlier retained resources.
Invalid paths and positions retain their current domain failures. No failure or
empty unsupported-symbol result changes workspace selection state.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select a qualified call or qualified function-value occurrence for a public function in an exported direct-dependency module. | Every selection returns the same sorted reference sites for that exact dependency function within the selected project, excluding the declaration. | Table-driven language-service and MCP adapter cases plus a saved MCP executable case. |
| Reach the same dependency module through each source form already admitted by saved project analysis. | Path, vendor, mirror, and locally materialized git dependencies produce the same reference boundary and only workspace `file:` locations. | Source-form matrix reusing captured dependency fixtures. |
| Use an import alias for the exact dependency module. | Alias-qualified calls and function values resolve to the dependency function; the written alias segment is not returned as a function reference. | Import-alias selection and reference cases. |
| Use equal function spellings in another dependency, another module, another selected project, a workspace declaration, a local binding, or a field. | Only occurrences with the selected dependency package, module, and declaration identity are returned. | Collision and other-project exclusion matrix. |
| Select a private, non-exported, invalid-casing, mismatched-package, transitive, standard-library, aliased, non-function, recovery, or absent symbol. | The request succeeds with an empty `references` array and does not expose a dependency source or reinterpret the selection. | Visibility and unsupported-origin decision table. |
| Request references for a source outside every selected project's captured owned-source set. | Anonymous single-file scope remains isolated and returns no dependency references. | Descendant-project and unrelated-source isolation cases. |
| Change the selected root identity, manifest, owned source, dependency manifest, dependency source, or dependency path set during capture. | Capture retries under the existing bound, then returns `snapshot_changed` without partial locations or publication. | Injected saved-capture change cases for the dependency reference operation. |
| Reach dependency resource capacity during the reference operation. | The operation returns `resource_capacity`, admits no new resource, returns no partial reference set, and preserves earlier resources and workspace selection. | Capacity-boundary state-preservation case. |

## Completion

This proposal is complete when every acceptance row passes and the MCP
specification and executable MCP examples state the implemented
direct-dependency function-reference boundary. Move completion history under
`../reference/implemented-proposals/`, remove this page from the Ready catalog,
and leave the umbrella proposal unselectable until another finite slice is
extracted.
