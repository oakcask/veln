---
role: proposal
update-when: The saved MCP reference symbol boundary, package type navigation, reference result schema, or planned package type-reference evidence changes.
---

# MCP Saved Package Type References

## Summary

Extend the existing MCP `references` tool to find selected-project uses of
public types from exported direct-dependency and embedded standard-library
modules. Keep the current unpaginated result schema and reference-only
workspace location boundary.

## Readiness

This slice is ready because its required foundations are implemented:

- saved project capture and workspace-wide reference collection;
- direct-dependency and standard-library package snapshot, definition, and
  source-resource publication;
- shared package type selection and type-reference collection; and
- stable `references` input, result, failure, and scope schemas.

The broader constructor, pagination, recovery, conformance, and plugin work in
[Agent Language Services](agent-language-services.md) is not required for this
slice.

## Scope

The selected symbol must be a public type declaration in an exported direct
dependency or embedded standard-library module. Selection must follow the same
saved-project import and implicit-prelude resolution used by definition lookup.

The result contains only resolved type reference sites in the selected
project's captured owned sources. These sites include type annotations, type
arguments, return types, aliases, and a type used as the qualifier of one of
its constructors. The result does not contain the package declaration or
package source-body occurrences. It retains the existing sorted `file:`
locations and project-wide scope metadata.

This slice does not add pagination, declaration inclusion, reference locations
inside package sources, constructor-symbol references, schema references,
public alias references, recovery symbols, or transitive-dependency
references.

This slice does not change the existing package function-reference boundary.
Selecting a supported public direct-dependency or standard-library function
continues to return its resolved project-source references as specified by
[MCP Workspace Projects, Resources, And Navigation](../specification/mcp.md#saved-workspace-navigation).

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select one exported direct-dependency type through qualified type uses in multiple project sources. | Return each resolved workspace occurrence once in canonical location order with project-wide scope. | Language-service dependency identity cases and one executable MCP specification case. |
| Select one exported standard-library type through explicit imports or accepted implicit-prelude paths. | Return qualified and implicit-prelude occurrences that resolve to the same standard-library declaration. | Language-service standard-library resolution cases and the executable MCP specification case. |
| Use the selected type as a constructor qualifier. | Include the type segment while leaving selection of the constructor symbol outside this slice. | Language-service type-reference cases and MCP adapter boundary cases. |
| Reuse the selected spelling for a workspace type, another package identity or module, constructor, value, field, string, or comment. | Exclude every occurrence that does not resolve to the selected package type. | Language-service collision table and MCP boundary cases. |
| Select a private type, a type from a non-exported module, a public type alias, or an invalid-casing type record. | Return an empty `references` array without widening the supported type set. | Language-service unsupported-type selection table and MCP success-boundary cases. |
| Select a constructor symbol, a schema, a package module segment, or a recovery symbol. | Preserve the existing successful empty `references` boundary for these unsupported symbol classes. | MCP unsupported-symbol success-boundary cases. |
| Select a supported public direct-dependency or standard-library function. | Preserve the existing package function-reference result instead of treating every non-type symbol as unsupported. | Existing MCP direct-dependency and standard-library package-function regression cases. |
| Change the selected project or package inputs during capture until retry exhaustion. | Return `snapshot_changed` without reference locations or scope metadata and without partial package resource admission. | Existing saved-reference capture and state-preservation harnesses extended with package type selection. |

## Completion

This proposal is complete when every acceptance row has checked evidence, the
MCP specification states the implemented package type boundary, and the
completed proposal record identifies the executable specification and unit-test
routes. The implementation must then remove this page from the active proposal
catalog.
