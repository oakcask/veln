---
role: proposal
update-when: Shared package definition selection, MCP definition results, package source resources, or planned package navigation evidence changes.
---

# MCP Package Definition Navigation

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

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select each supported public declaration class from an exported direct-dependency module. | `definition` returns its canonical dependency `veln-pkg:` URI and exact declaration range. | Table-driven MCP adapter cases for functions, types, constructors, schemas, and public member aliases. |
| Select an implicit-prelude or explicitly imported public standard-library declaration. | `definition` returns the canonical embedded `std` source URI and exact declaration range. | Table-driven standard-library cases covering implicit and explicit imports. |
| Use direct path, vendor, mirror, and locally available git inputs with byte-identical package snapshots. | Results use the same identity-and-digest URI form and contain no physical source kind or materialization path. | Source-kind matrix reusing captured dependency fixtures. |
| Select a private declaration, a declaration in a non-exported or invalid-cased source, an invalid declaration, or a mismatched package import. | The request succeeds with `definition: null` and does not reinterpret the selection. | Visibility, casing, and exact-import rejection matrix. |
| Read the URI returned by `definition`. | The listed resource returns the exact captured source bytes, and the result range addresses the selected declaration token. | Definition-to-resource round trip with LF, CRLF, and non-ASCII text. |
| Edit or remove the physical dependency after the package location is returned. | The retained URI continues to return the original bytes and range target until server shutdown. | Saved-snapshot lifetime transition case. |
| Change a manifest or included source byte in a later stable capture. | A later definition uses the new snapshot URI while the earlier URI remains readable. | Same-identity, different-digest coexistence case. |
| Exhaust package resource capacity during a dependency definition operation. | The operation returns `resource_capacity`, no definition or new resource is published, and prior resources remain unchanged. | Capacity-boundary and atomic state-preservation case. |
| Change captured inputs during the operation. | Stable capture retries under the existing bound, then returns `snapshot_changed` with no partial definition or resource admission. | Injected capture-change case. |
| Select a package declaration and request references, prepare-rename, or rename through existing adapters. | MCP exposes no package reference or mutation capability in this slice; existing LSP package mutation rejection remains unchanged. | MCP schema checks and existing LSP package-boundary cases. |

## Completion

This proposal is complete when every acceptance row passes and the MCP
specification and executable MCP examples state the implemented package
definition and source-resource relationship. Move completion history under
`../reference/implemented-proposals/`, remove this page from the Ready catalog,
and leave the umbrella proposal unselectable until another finite slice is
extracted.
