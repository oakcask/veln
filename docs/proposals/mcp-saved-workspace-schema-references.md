---
role: proposal
update-when: The saved MCP reference symbol boundary, workspace schema navigation, reference result schema, or planned workspace schema-reference evidence changes.
---

# MCP Saved Workspace Schema References

## Summary

Extend the existing MCP `references` tool to find selected-project uses of a
workspace schema in `decode` and `encode` expressions. Keep the current
unpaginated result schema and project-owned source-location boundary.

## Readiness

This slice is ready because its required foundations are implemented:

- saved project capture and workspace-wide navigation;
- workspace schema declaration and `decode` and `encode` path selection;
- stable `references` input, result, failure, and scope schemas; and
- canonical workspace location ordering and stable-capture failure handling.

The package-schema, public-alias, pagination, recovery, conformance, and plugin
work in [Agent Language Services](agent-language-services.md) is not required
for this slice.

## Scope

The selected symbol must be a schema declaration owned by the inferred saved
project. Selection must follow the existing module import, visibility, exact
test-companion, and shadowing rules used by definition lookup.

The result contains only schema path-leaf occurrences in `decode` and `encode`
expressions in the selected project's captured owned sources. It includes
accepted local, imported, and module-qualified forms that resolve to the same
workspace schema. It excludes module qualifiers, the schema declaration, and
same-spelled paths that resolve to another declaration. It retains the
existing sorted `file:` locations and project-wide scope metadata.

This slice does not add package-schema references, schema composition or
public schema-alias references, package or workspace public-alias traversal,
pagination, declaration inclusion, recovery symbols, casing-neutral
selection, transitive dependency references, or rename behavior.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select one local workspace schema used by both `decode` and `encode` expressions in multiple project sources. | Return each resolved schema path leaf once in canonical location order with project-wide scope. | Language-service schema identity cases and one executable MCP specification case. |
| Select one visible schema from an imported workspace module through accepted imported and module-qualified forms. | Return only uses that resolve to that declaration under the saved project import and visibility rules. | Language-service import and visibility cases plus MCP adapter cases. |
| Select a private target schema from its exact test companion. | Include the companion's resolved uses without making the schema visible to unrelated modules or test companions. | Language-service companion-boundary cases and MCP project-scope cases. |
| Reuse the schema spelling for another module's schema, a function, type, constructor, value, field, operation, string, or comment. | Exclude every occurrence that does not resolve to the selected workspace schema. | Language-service collision table and MCP boundary cases. |
| Select a module qualifier, package schema, public schema alias, schema-composition target, recovery symbol, invalid-casing record, or another unsupported symbol class. | Return an empty `references` array without widening the supported symbol set. | Language-service unsupported-selection table plus MCP success-boundary cases. |
| Request the same selection from an anonymous source or a source outside the inferred selected project. | Preserve single-file isolation and return no project-wide workspace schema references. | Descendant-project and unrelated-source isolation cases. |
| Change the selected project inputs during capture until retry exhaustion. | Return `snapshot_changed` without reference locations or scope metadata. | Existing saved-reference capture and state-preservation harness extended with workspace schema selection. |

## Completion

This proposal is complete when every acceptance row has checked evidence, the
MCP specification states the implemented workspace schema-reference boundary,
and the completed proposal record identifies the executable specification and
unit-test routes. The implementation must then remove this page from the active
proposal catalog.
