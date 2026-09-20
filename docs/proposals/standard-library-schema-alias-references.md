---
role: proposal
update-when: Standard-library schema or direct-dependency schema-alias navigation, package declaration inclusion, reference pagination, or planned standard-library schema-alias evidence changes.
---

# Standard-Library Schema-Alias References

## Outcome And Readiness

Let editor and agent clients find the selected-project uses of an eligible
public schema alias from the retained standard library. Preserve the alias as a
separate navigation identity from its target schema. This is a selectable
slice of [Agent Language Services](agent-language-services.md).

The shared language service already indexes standard-library schemas and
direct-dependency schema aliases. It also resolves standard-library module
imports, package virtual locations, schema composition leaves, and schema
operations. LSP and MCP already adapt the same saved navigation result. MCP
already supports optional package declaration inclusion, deterministic
pagination, stable snapshot capture, and standard-library resource
publication. This slice therefore requires no new syntax, package origin,
transport field, cursor rule, or workspace-selection rule.

## Current Boundary

[Editor Support](../specification/editor-support.md#lsp-navigation-formatting-and-rename)
and
[MCP Workspace Projects, Resources, And Navigation](../specification/mcp.md#saved-workspace-navigation)
currently define standard-library schema references and direct-dependency
schema-alias references. They keep standard-library schema aliases as
successful empty selections. Focused language-service, LSP, and MCP tests
retain that exclusion.

This proposal changes only references for the schema-alias symbol class.
Definition, rename, package-source reference search, transitive dependency
navigation, recovery navigation, and casing-neutral navigation remain outside
this slice.

## Proposed Contract

An eligible alias is public and declared in an exported standard-library
source. Its written target resolves through a finite, acyclic chain. Each
non-terminal hop resolves uniquely to a public schema alias in the retained
standard-library snapshot. The final hop resolves uniquely to a public schema
declared in an exported standard-library source. Bare, fully qualified, and
valid unique implicit-module target paths use the existing package lookup
rules. A prelude alias additionally uses the existing bare prelude visibility
rule.

The selected alias has its own declaration identity. Selecting a resolved
consumer leaf returns only selected-project source occurrences that resolve to
that alias. Direct schema uses, target expressions, sibling aliases, and
package implementation occurrences do not join the result.

The following table is the planned acceptance model. It does not claim that
standard-library schema-alias references are implemented.

| Input or event | Required observation | Planned evidence |
| --- | --- | --- |
| Select an eligible alias through `decode`, `encode`, a direct composition field, a valid `Repeat` payload, or an array payload. | Return every matching selected-project occurrence with one alias identity and project-wide scope. | Shared language-service matrix plus paired LSP and MCP exact-location cases. |
| Select the alias through its full module path, unique implicit leaf module, explicit import path, or valid prelude bare form. | Resolve each supported spelling to the same alias identity. Exact imports retain the existing precedence over implicit module leaves. | Table-driven module, import, prelude, and source-order cases. |
| Select the target schema or another alias that reaches the same schema. | Keep each selected identity and its reference set separate. Do not merge aliases with the terminal schema or with one another. | Alias-chain identity matrix with separately asserted unions. |
| Request MCP references with declaration inclusion enabled. | Add the selected alias's one canonical `veln-pkg:` declaration before the existing sort and pagination. LSP and declaration-disabled MCP results contain only selected-project `file:` locations. | Paired adapter cases and a standard-library resource round trip. |
| The alias is private, declared in a non-exported source, invalid-cased, ambiguous, recovered, cyclic, or has an unresolved, wrong-kind, private, non-exported, or transitive target. | Return the existing successful empty result. An ineligible alias blocks fallback to a same-spelled schema instead of becoming another identity. | Eligibility and target-resolution decision table covering every listed boundary. |
| Imports are duplicate, conflicting, mismatched, or syntax-recovered. | Preserve the current deterministic empty or exact-import result independently of import order. | Paired import-order and recovery cases for composition and operation leaves. |
| The selection is an import token, module qualifier, alias-target expression, package-source occurrence, comment, string, field name, or same-spelled unrelated declaration. | Do not select or add the alias identity. | Lexical and semantic exclusion matrix. |
| Saved source contains non-BMP text or an LSP overlay adds one alias use. | LSP converts UTF-16 positions and observes the overlay. MCP keeps the saved Unicode-scalar baseline. Normalized saved locations otherwise agree. | Paired non-BMP adapter case with one overlay-only LSP location. |
| An MCP result spans pages. | Preserve the initial scope and declaration policy across pages. Concatenated pages equal the unpaged sorted result without gaps or duplicates. | Pagination transition case with the declaration on a noninitial page. |
| Stable capture fails. | Return the existing domain failure without partial references, scope, declaration, cursor, or resource-state mutation. | Focused MCP capture-failure and state-preservation case. |

An anonymous single-file request retains its current scope. It can return uses
from only that captured file.

## Evidence And Completion

Extend the shared package-schema navigation matrix first. Reuse the existing
direct-dependency schema-alias expected roles and the standard-library schema
origin cases instead of creating an adapter-specific lookup model. Add paired
LSP and MCP cases for exact saved locations, one LSP overlay, declaration
policy, and package identity. Add focused MCP pagination and capture-failure
cases. The injected standard-library snapshot needed by these cases is not
available to the checked examples harness, so direct shared-service and
adapter tests are the primary executable evidence.

Completion requires every acceptance row to pass and the current alias
eligibility, identity, occurrence, declaration, adapter-coordinate,
pagination, and state-preservation contracts to appear in `editor-support.md`
and `mcp.md` under `docs/specification/`. Remove this proposal and its Ready
entry after those conditions hold. Keep transitive dependencies, recovery,
casing-neutral navigation, full cross-adapter conformance, agent-skill,
conformance-gate, and plugin work in the umbrella.
