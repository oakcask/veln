---
role: proposal
update-when: The saved MCP reference symbol boundary, package constructor navigation, reference result schema, or planned package constructor-reference evidence changes.
---

# MCP Saved Package Constructor References

## Summary

Extend the existing MCP `references` tool to find selected-project uses of
public constructors from exported direct-dependency and embedded
standard-library modules. Keep the current unpaginated result schema and
reference-only workspace location boundary.

## Readiness

This slice is ready because its required foundations are implemented:

- saved project capture and workspace-wide reference collection;
- direct-dependency and standard-library package snapshot, definition, source,
  and documentation resource publication;
- shared package constructor selection, semantic identity, and reference
  collection; and
- stable `references` input, result, failure, and scope schemas.

The broader schema, public-alias, pagination, recovery, conformance, and plugin
work in [Agent Language Services](agent-language-services.md) is not required
for this slice.

## Scope

The selected symbol must be a public constructor of a public type in an
exported direct-dependency or embedded standard-library module. Selection must
follow the same saved-project import, implicit-prelude, visibility, and
shadowing rules used by definition lookup.

The result contains only constructor occurrences in the selected project's
captured owned sources. It includes constructor calls and constructor patterns,
including qualified and accepted bare forms that resolve to the same package
constructor. It excludes the owning type qualifier and the package declaration
and source-body occurrences. It retains the existing sorted `file:` locations
and project-wide scope metadata.

This slice does not add pagination, declaration inclusion, reference locations
inside package sources, type-reference widening, schema references, public
alias selections or re-export traversal, recovery symbols, transitive
dependency references, or rename behavior.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select one public constructor through qualified calls and patterns in multiple project sources. | Return each resolved workspace constructor occurrence once in canonical location order with project-wide scope. | Language-service direct-dependency identity cases and one executable MCP specification case. |
| Select one public embedded standard-library constructor through accepted explicit-import or implicit-prelude forms. | Return qualified and bare calls and patterns that resolve to the same standard-library constructor. | Language-service standard-library resolution cases and the executable MCP specification case. |
| Qualify a constructor as `import::Type::Constructor` or `import::Constructor`. | Select only the constructor segment; leave the type segment governed by the implemented package type-reference contract. | Shared navigation qualifier cases and MCP adapter selection cases. |
| Reuse the constructor spelling for another package identity, module, type, constructor, function, value, field, operation, string, or comment. | Exclude every occurrence that does not resolve to the selected package constructor. | Language-service collision table and MCP boundary cases. |
| Select a private constructor, a constructor of a private type, a constructor from a non-exported module, an invalid-casing constructor, a public-alias route, a package module or type segment, a recovery symbol, or another unsupported symbol class. | Return an empty `references` array without widening the supported symbol set. | Language-service visibility and unsupported-selection table plus MCP success-boundary cases. |
| Request the same selection from an anonymous source or a source outside the inferred selected project. | Preserve single-file isolation and return no package constructor references. | Descendant-project and unrelated-source isolation cases. |
| Change the selected project or package inputs during capture until retry exhaustion. | Return `snapshot_changed` without reference locations or scope metadata and without partial package resource admission. | Existing saved-reference capture and state-preservation harnesses extended with package constructor selection. |

## Completion

This proposal is complete when every acceptance row has checked evidence, the
MCP specification states the implemented package constructor boundary, and the
completed proposal record identifies the executable specification and unit
test routes. The implementation must then remove this page from the active
proposal catalog.
