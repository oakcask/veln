---
role: implementation-record
update-when: Shared package definition selection, MCP definition results, package source resources, or executable package navigation evidence changes.
---

# MCP Package Definition Navigation

This record preserves the completed proposal history for returning retained
package source locations from the MCP `definition` tool. Current behavior is
specified by [MCP saved workspace navigation](../../specification/mcp.md#saved-workspace-navigation)
and checked by the `definition-package-navigation` executable MCP case plus
focused `veln-mcp` adapter tests.

## Summary

Return canonical `veln-pkg:` source locations from the existing MCP
`definition` tool when a saved workspace source selects a visible declaration
in a direct dependency or the embedded standard library. This slice connects
the shared package navigation result to package source resources already
retained by the MCP server. It does not add package references, documentation
links, rename, or new source mutation.

## Scope

| Included | Excluded |
| --- | --- |
| Public declarations in exported direct-dependency and standard-library modules that the shared language service already selects. | Private declarations, non-exported modules, transitive dependencies, and unsupported symbol classes. |
| Canonical package source URI and one-based Unicode-scalar declaration range in the existing definition result. | Package documentation URI fields and documentation resource publication. |
| Direct path, vendor, mirror, locally available git, and embedded standard-library snapshots already captured by saved analysis. | Dependency discovery, remote materialization, registry resolution, and changes to package snapshot capture. |
| Exact source-resource round trips and operation-atomic dependency admission. | Package reference search, pagination, cursors, prepare-rename, rename edits, and client plugins. |

The language service remains authoritative for symbol selection, package
visibility, declaration identity, and the package-relative range. The package
virtual-source catalog remains authoritative for URI spelling and exact source
resolution. This proposal defines only the MCP adapter result and its
relationship to the server's retained source-resource state.

## Selection Contract

The MCP server analyzes the same immutable saved-project capture used by the
current `definition` operation. A selected direct-dependency declaration is
eligible only when all of these facts hold:

- the source writes the exact external package and module import required by
  current name resolution;
- the captured dependency identity matches that import;
- the declaration's source belongs to the dependency's validated export set;
- the declaration is public and has no invalid source identifier casing
  record; and
- the shared language service selects the declaration for the requested
  position.

The embedded standard library follows the same exported-source and public
declaration boundary. Implicit prelude lookup and an exact explicit standard
import can select a supported declaration. A workspace declaration continues
to return its current canonical `file:` URI. A valid position with no eligible
symbol returns `definition: null`.

This slice exposes the package-backed function, type, constructor, schema, and
public member-alias classes already represented by shared definition
selection. It does not reinterpret an import module segment, recovery record,
private declaration, or unsupported occurrence as another symbol class.

## Result And Resource Contract

For an eligible package declaration, `definition.uri` is the exact canonical
`veln-pkg:` URI carried by the shared navigation location. The result does not
reconstruct the URI from a materialization path and does not expose that path.
The half-open range identifies the declaration token and uses the existing MCP
one-based line and Unicode-scalar column contract.

The returned URI must identify a source resource retained by the same MCP
server. `resources/read` for that exact URI returns the complete captured UTF-8
source text, including original line endings. The returned range addresses the
same declaration token in those bytes. Unknown or noncanonical package URIs
continue to return `resource_not_found` without normalization or filesystem
fallback.

A successful direct-dependency definition operation admits the dependency
snapshot through the implemented atomic resource admission contract before it
returns the package location. If resource admission would exceed capacity, the
operation returns `resource_capacity`, publishes no definition, and preserves
all earlier resources. Capture or analysis failure also publishes no partial
definition or package resource state. The embedded standard snapshot is
already retained at server startup.

Package locations are immutable and read-only. This slice returns no package
reference locations, prepare-rename range, rename edits, or documentation URI.
Workspace refresh or later dependency changes do not mutate a package source
resource that was already returned.

## Completion Evidence

| Case | Evidence |
| --- | --- |
| Public declaration classes in exported direct-dependency modules return canonical dependency `veln-pkg:` locations and exact declaration ranges. | `definition_resolves_public_package_symbol_classes` covers functions, types, constructors, schemas, and public function aliases. |
| Implicit-prelude and explicitly imported public standard-library declarations return canonical embedded `std` source locations. | `definition_resolves_implicit_and_explicit_standard_library_symbols` |
| Direct path, vendor, mirror, and locally available git inputs use the identity-and-digest URI form without source-kind or materialization-path leakage. | `definition_dependency_package_uris_are_independent_of_source_kind` |
| Private declarations, non-exported sources, invalid declaration casing, mismatched package imports, and module-segment selections return `definition: null`. | `definition_rejects_ineligible_package_selections_without_reinterpreting_them` |
| A returned package definition URI resolves through retained source resources to exact captured source bytes and the declaration range addresses the selected token, including CRLF and non-ASCII UTF-8 package source text. | `definition_resolves_public_package_symbol_classes`, `definition_resolves_implicit_and_explicit_standard_library_symbols`, `definition_round_trips_crlf_non_ascii_package_source`, and `definition-package-navigation` |
| Editing or removing a physical dependency after returning a package location does not mutate the retained resource. | `definition_retains_package_snapshot_bytes_across_dependency_changes` |
| A later stable capture with changed dependency source bytes receives a new snapshot URI while the earlier URI remains readable. | `definition_retains_package_snapshot_bytes_across_dependency_changes` |
| Dependency definition resource-capacity failure is atomic and preserves prior resources. | `saved_project_capacity_failures_match_advertised_result_schemas` |
| Changed captured inputs fail with `snapshot_changed` and no success-only definition member. | `definition_rejects_paths_and_changed_workspace_identity` and stable-capture unit coverage |
| MCP exposes package definition locations without package reference or mutation capability. | `definition-package-navigation`, `references-workspace`, and existing LSP package-boundary tests |
