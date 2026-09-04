---
role: proposal
update-when: MCP package-documentation resource publication, package documentation catalogs, package snapshot admission, or definition documentation links change.
---

# MCP Package Documentation Resources

## Outcome

Finish publishing the existing package-documentation catalog through `veln
mcp` for admitted direct-dependency snapshots and package-backed definition
results. A client can follow a documentation URI returned by definition lookup
and read the exact immutable documentation resource without access to a
physical package cache path.

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

The embedded standard-library index, module, declaration, template, exact-read,
status-only, and rejection boundary is implemented in
[MCP Workspace Projects And Navigation](../specification/mcp.md). This
remaining proposal adds direct-dependency publication and definition-result
links for the same package-documentation result. It does not change catalog
generation semantics or identity.

## Scope

| Included | Excluded |
| --- | --- |
| Documentation resources for each successfully admitted direct-dependency snapshot. | Workspace-package documentation and arbitrary `veln doc` invocation. |
| Direct-dependency Markdown index, module, declaration, and status resources projected from the existing catalog. | A new package-documentation catalog schema or identity. |
| Bounded direct-dependency index or status metadata in `resources/list`, exact `resources/read` access, and reuse of the implemented package-documentation resource templates. | Package and standard-library search through `search_docs` or reads through `read_doc`. |
| Documentation links on package-backed definition results when the selected declaration has a published resource. | Dependency reference expansion, pagination, rename, formatting, or mutation. |
| Snapshot retention and capacity behavior shared with existing package source resources. | Resource subscriptions, completion, hover, and client plugin packaging. |

## Resource Contract

For each retained direct-dependency package snapshot, the MCP adapter
generates the existing package-documentation result from that same captured
snapshot. A successful result publishes its index, module, and declaration
Markdown resources. A failed result publishes only its status Markdown
resource. Package source resources remain available in either case.

The renderer is a pure projection of the immutable result. The index preserves
the catalog metadata and ordered module links. A module resource preserves its
documentation and ordered public declaration links. A declaration resource
preserves its kind, signature, documentation, contracts, constructors,
doctests, expected outputs, and resolved documentation links when those fields
exist in the catalog. A status resource preserves the ordered gate, code,
message, and optional source span of each diagnostic. Rendering does not add
raw manifests, physical paths, dependency selectors, environment values, or
other data excluded from the catalog.

`resources/list` returns one package-documentation index for each successful
result or one status resource for each failed result. It returns these entries
in URI byte order together with existing resources. Each entry contains the
exact URI, stable name, title, and Markdown media type. It contains a
description only when the package-documentation renderer supplies one. The
metadata exposes only the allowlist defined by the package-documentation
specification.

`resources/templates/list` advertises the canonical module and declaration
resource forms. The index supplies exact resource URIs; clients do not
construct identifiers from template variables. Module and declaration
resources are not enumerated eagerly in `resources/list`.

`resources/read` accepts an exact canonical package-documentation URI retained
by the server and returns its complete Markdown text and media type. An
unknown, noncanonical, wrong-snapshot, wrong-documentation-digest, or
unpublished URI returns the existing `resource_not_found` domain failure. The
adapter does not normalize the URI, regenerate from a newer snapshot, or fall
back to a physical path.

Direct-dependency documentation becomes available atomically with the existing
successful snapshot admission path. Re-admitting the same package identity and
snapshot digest does not duplicate resources. A different digest for the same
package identity can coexist until server shutdown.

If admitting a new package snapshot would exceed the existing retained-package
capacity, the operation returns `resource_capacity` before publishing source
or documentation resources for that snapshot. Existing resources remain
readable. Workspace refresh or later filesystem changes do not remove or
change a retained documentation resource.

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
| Admit a direct dependency whose documentation generation succeeds. | Source resources and the documentation index appear atomically in URI byte order; index-linked resources become readable and repeated admission adds no duplicates. | Path-dependency admission and repeated-admission cases. |
| Read a published index, module, or declaration URI. | The response contains the complete Markdown bytes and media type rendered from the admitted catalog. | Route-equality cases against the transport-independent package-documentation result. |
| Render catalog metadata, documentation, signatures, contracts, constructors, doctests, expected outputs, references, or status diagnostics. | The Markdown preserves the corresponding ordered semantic fields and does not expose catalog-excluded metadata. | Renderer projection and disclosure-boundary cases. |
| Resolve an exported dependency or standard-library declaration with published documentation. | The definition result contains the exact readable declaration URI from the same snapshot and documentation digest. | Definition-to-documentation round-trip cases, including a constructor-to-type case. |
| Generate a status-only documentation result. | Only the status documentation resource is listed and readable; no index, module, declaration, or definition documentation link is readable, while source resources remain readable. | Parse, manifest, documentation-reference, and doctest-gate failure table. |
| Read a noncanonical, unknown, wrong-snapshot, wrong-documentation-digest, or unpublished URI. | The server returns `resource_not_found` without content, normalization, regeneration, or filesystem fallback. | URI rejection table and stdio domain-failure case. |
| Change admitted package bytes and admit the new snapshot. | Old and new documentation URIs coexist and retain their original bytes until shutdown. | Snapshot-coexistence and retained-read cases. |
| Reach retained-package capacity and attempt one more admission. | The operation returns `resource_capacity`; no resource from the rejected snapshot appears and all prior resources remain unchanged. | Capacity atomicity and state-preservation case. |
| Refresh the workspace or change dependency files after publication. | Previously returned documentation metadata and bytes remain unchanged and readable. | Lifecycle state-preservation case. |

## Completion

This proposal is complete when every acceptance row has executable MCP
coverage, the stdio specification case checks the public resource and
definition-link surface, and the MCP specification states the implemented
package-documentation resource boundary. Move the completed record out of this
directory at that time.
