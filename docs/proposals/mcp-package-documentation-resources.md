---
role: proposal
update-when: MCP package-backed definition documentation links, package documentation catalogs, or package snapshot admission change.
---

# MCP Package Definition Documentation Links

## Outcome

Finish returning package-documentation declaration URIs from package-backed
`definition` results. A client can follow a documentation URI returned by
definition lookup and read the exact immutable documentation resource without
access to a physical package cache path.

## Existing Foundations

The required transport-independent behavior is already implemented and
specified:

- [Package Documentation Catalogs](../specification/package-documentation.md)
  defines package-atomic generation, canonical `veln-doc:` URIs, catalog
  content, status-only failures, and declaration-location lookup.
- [Package Snapshot Digests](../specification/package-snapshots.md) and
  [Package Virtual Sources](../specification/package-virtual-sources.md)
  define immutable admitted snapshots and canonical package identities.
- [MCP Workspace Projects And Navigation](../specification/mcp.md) defines
  package snapshot admission, retained source resources, resource capacity,
  and package-backed definition results.

The embedded standard-library and direct-dependency package-documentation
resource publication boundary is implemented in
[MCP Workspace Projects And Navigation](../specification/mcp.md). This
remaining proposal adds definition-result links for the same retained
package-documentation result. It does not change catalog generation semantics,
resource publication, or identity.

## Scope

| Included | Excluded |
| --- | --- |
| Documentation links on package-backed definition results when the selected declaration has a published resource. | Dependency reference expansion, pagination, rename, formatting, or mutation. |
| Constructor selections link to the owning type documentation when that owning type has a published declaration resource. | Workspace-package documentation and arbitrary `veln doc` invocation. |
| Link omission when the retained package-documentation result is status-only, the declaration is unpublished, or the selected location is not from the same package snapshot. | A new package-documentation catalog schema, identity, renderer, or MCP resource publication behavior. |
| Definition result schema and executable MCP evidence for the new optional documentation URI. | Package and standard-library search through `search_docs` or reads through `read_doc`. |

## Definition Link Contract

A package-backed definition result includes the exact declaration
documentation URI when the selected public declaration resolves through the
same successful package-documentation result. A constructor selection links to
the owning type documentation according to the existing catalog lookup.

The result omits the documentation URI when generation produced a status-only
failure, the declaration is not published, or the selected location does not
belong to that package snapshot. The definition location itself and the source
resource remain unchanged.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Resolve an exported dependency or standard-library declaration with published documentation. | The definition result contains the exact readable declaration URI from the same snapshot and documentation digest. | Definition-to-documentation round-trip cases, including a constructor-to-type case. |
| Resolve a constructor selection with published documentation. | The definition result contains the owning type declaration documentation URI from the same snapshot and documentation digest. | Constructor-to-type definition documentation round-trip case. |
| Resolve a package declaration whose documentation result is status-only. | The definition result omits the documentation URI while retaining the package source location. | Status-only definition omission case. |
| Resolve a package declaration that is not published in the package-documentation catalog. | The definition result omits the documentation URI while retaining the package source location. | Private, non-exported, and unsupported selection omission cases. |
| Resolve a package location from another snapshot or a stale retained URI. | The definition result omits the documentation URI instead of recomputing or normalizing documentation identity. | Same-identity snapshot coexistence and stale-location cases. |

## Completion

This proposal is complete when every remaining acceptance row has executable
MCP coverage, the stdio specification case checks the definition-link surface,
and the MCP specification states the implemented package-backed documentation
link boundary. Move the completed record out of this directory at that time.
