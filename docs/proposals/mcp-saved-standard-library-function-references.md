---
role: proposal
update-when: The saved MCP reference symbol boundary, embedded standard-library navigation, reference result schema, or planned standard-library function-reference evidence changes.
---

# MCP Saved Standard-Library Function References

## Summary

Extend the existing MCP `references` tool to find selected-project uses of
public functions from exported embedded standard-library modules. Keep the
current unpaginated result schema and function-only package boundary.

## Readiness

This slice is ready because its required foundations are implemented:

- saved project capture and workspace-wide reference collection;
- embedded standard-library snapshot, definition, source-resource, and
  package-documentation publication;
- direct-dependency function-reference selection and MCP adaptation; and
- stable `references` input, result, failure, and scope schemas.

The broader navigation, pagination, conformance, and plugin work in
[Agent Language Services](agent-language-services.md) is not required for this
slice.

## Scope

The selected symbol must be a public function declaration in an exported
embedded standard-library module. Selection may use the implicit `prelude`
module or an explicit import from `std`, and must follow the same saved-project
name resolution used by definition lookup.

The result contains only reference sites in the selected project's captured
owned sources. It does not contain the standard-library declaration or
standard-library source-body occurrences. It retains the existing sorted
`file:` locations and project-wide scope metadata.

This slice does not add pagination, declaration inclusion, reference locations
inside package sources, public function-alias references, non-function package
symbols, recovery symbols, or casing-neutral invalid symbols.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select the same exported standard-library function through qualified calls and qualified function-value occurrences in multiple project sources. | Return each resolved workspace occurrence once in canonical location order with project-wide scope. | Language-service identity cases and one executable MCP specification case. |
| Select an implicitly available prelude function through its accepted qualified and unqualified forms. | Return both forms when they resolve to the same standard-library declaration. | Prelude selection and MCP adapter cases. |
| Reuse the selected spelling for a workspace declaration, direct-dependency declaration, lexical binding, field, string, or comment. | Exclude every occurrence that does not resolve to the selected standard-library function. | Language-service collision table and MCP boundary cases. |
| Select a private function, non-exported module function, public function alias, non-function package symbol, package module segment, recovery symbol, or invalid-casing record. | Return an empty `references` array without widening the supported symbol set. | Language-service unsupported-selection table and MCP success-boundary cases. |
| Change the selected project source set or bytes during capture until retry exhaustion. | Return `snapshot_changed` without reference locations or scope metadata. | Existing saved-reference capture harness extended with a standard-library selection. |

## Completion

This proposal is complete when every acceptance row has checked evidence, the
MCP specification states the implemented standard-library function boundary,
and the completed proposal record identifies the executable specification and
unit-test routes. The implementation must then remove this page from the active
proposal catalog.
