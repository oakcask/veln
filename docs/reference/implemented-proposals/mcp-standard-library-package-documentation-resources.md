---
role: implementation-record
update-when: Embedded standard-library package-documentation MCP resources, resource templates, Markdown rendering, executable MCP evidence, or current MCP and package-documentation specifications change.
---

# MCP Standard Library Package Documentation Resources

This record preserves the completed slice that publishes embedded `std`
package-documentation Markdown resources through MCP. Current behavior is
specified by [MCP resources](../../specification/mcp.md#resources) and
[Package Documentation Catalogs](../../specification/package-documentation.md).
The remaining direct-dependency package-documentation and definition-link work
stays under
[MCP Package Documentation Resources](../../proposals/mcp-package-documentation-resources.md).

## Completed Boundary

The repository generates a checked package-documentation resource bundle from
the embedded `std` package snapshot. Server startup validates the bundle digest,
requires its snapshot digest to match the retained snapshot, and publishes its
resources without rerunning package analysis. A successful result publishes
one listed index resource and exact linked module and declaration resources. A
failed result publishes one listed status resource instead. Module and
declaration resources are readable only by exact URI and are advertised by
`resources/templates/list`; they are not eagerly listed by `resources/list`.

The Markdown renderer preserves the catalog metadata, module links,
declaration links, documentation, signatures, contracts, constructors,
doctests, expected outputs, references, and status diagnostics that exist in
the package-documentation result. It does not expose raw manifests, physical
paths, dependency selectors, environment values, or fields excluded from the
catalog.

This slice does not publish direct-dependency documentation resources, attach
documentation URIs to package-backed definition results, add package search,
change `veln doc`, or change package-documentation catalog identity.

## Completion Evidence

| Contract | Evidence |
| --- | --- |
| Initialization lists the embedded `std` documentation index, advertises package-documentation resource templates, and reads index-linked module and declaration resources exactly. | `standard-library-package-documentation-resources` executable MCP case and `standard_library_package_documentation_round_trips_from_index_links` |
| Module and declaration resources are omitted from `resources/list` while exact reads remain available. | `standard-library-package-documentation-resources` executable MCP case and `standard_library_package_documentation_round_trips_from_index_links` |
| Malformed template-list parameters and unpublished, noncanonical, wrong-snapshot, wrong-documentation-digest, and missing package-documentation URIs fail through the specified protocol errors. | `standard-library-package-documentation-resources`, `resource_templates_list_advertises_package_documentation_forms`, and `resources_reject_malformed_params_and_unknown_uris` |
| Markdown rendering preserves ordered catalog fields and status diagnostics while enforcing the catalog disclosure boundary. | `successful_rendering_preserves_ordered_fields_and_links` and `status_rendering_preserves_diagnostics_and_disclosure_boundary` |
| A failed embedded `std` package-documentation result publishes only a listed status document. | `standard_library_documentation_failure_publishes_only_status_documentation` |
| Startup loads the digest-validated checked bundle for the exact embedded snapshot, while generator or renderer changes cannot leave it stale. | `checked_standard_library_resources_load_the_prebuilt_documentation_bundle` and the `veln-repo-mcp-standard-library-docs` freshness check |
