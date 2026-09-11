---
role: proposal
update-when: The saved MCP reference symbol boundary, package public function-alias navigation, reference result schema, or planned package function-alias evidence changes.
---

# MCP Saved Package Function-Alias References

## Summary

Extend the existing MCP `references` tool to find selected-project uses of a
public function alias from an exported direct-dependency or embedded
standard-library module. Keep the current unpaginated result schema and
reference-only workspace location boundary.

## Readiness

This slice is ready because its required foundations are implemented:

- saved project capture and workspace-wide reference collection;
- direct-dependency and standard-library package snapshot, definition, source,
  and documentation resource publication;
- package public function-alias selection and semantic identity; and
- stable `references` input, result, failure, and scope schemas.

The broader schema, public type-alias, public schema-alias, pagination,
recovery, conformance, and plugin work in
[Agent Language Services](agent-language-services.md) is not required for this
slice.

## Scope

The selected symbol must be a public function alias in an exported
direct-dependency or embedded standard-library module. Selection must follow
the same saved-project import, implicit-prelude, visibility, and shadowing
rules used by definition lookup.

The result contains only occurrences that resolve to the selected alias in the
selected project's captured owned sources. It includes qualified calls and
qualified function-value occurrences, plus accepted bare implicit-prelude
forms. It excludes the alias declaration, the alias target, occurrences of the
target function, package source bodies, and equal spellings that resolve to a
different declaration. It retains the existing sorted `file:` locations and
project-wide scope metadata.

Selecting the target function remains a distinct query and does not include
uses of its public alias. This slice does not add public type-alias or public
schema-alias references, alias-chain traversal, pagination, declaration
inclusion, package-source reference locations, recovery symbols, transitive
dependency references, casing-neutral selection, or rename behavior.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select one exported direct-dependency public function alias through qualified calls and function-value uses in multiple project sources. | Return each resolved alias occurrence once in canonical location order with project-wide scope. | Language-service dependency alias-identity cases and one executable MCP specification case. |
| Select one exported embedded standard-library public function alias through an explicit import or accepted implicit-prelude form. | Return qualified and accepted bare occurrences that resolve to the same standard-library alias. | Language-service standard-library alias-resolution cases and the executable MCP specification case. |
| Select the alias target or use the target function directly. | Keep target-function references separate from alias references in both directions. | Language-service target-and-alias identity matrix and MCP adapter cases. |
| Reuse the alias spelling for another package identity, module, function, type, constructor, value, field, operation, string, or comment. | Exclude every occurrence that does not resolve to the selected package alias. | Language-service collision table and MCP boundary cases. |
| Select a private alias, an alias from a non-exported module, an alias chain, a public type or schema alias, an invalid-casing alias, a recovery symbol, a package module segment, or another unsupported symbol class. | Return an empty `references` array without widening the supported symbol set. | Language-service visibility and unsupported-selection table plus MCP success-boundary cases. |
| Request the same selection from an anonymous source or a source outside the inferred selected project. | Preserve single-file isolation and return no package alias references. | Descendant-project and unrelated-source isolation cases. |
| Change the selected project or package inputs during capture until retry exhaustion. | Return `snapshot_changed` without reference locations or scope metadata and without partial package resource admission. | Existing saved-reference capture and state-preservation harnesses extended with package function-alias selection. |

## Completion

This proposal is complete when every acceptance row has checked evidence, the
MCP specification states the implemented package public function-alias
boundary, and the completed proposal record identifies the executable
specification and unit-test routes. The implementation must then remove this
page from the active proposal catalog.
