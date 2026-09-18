---
role: proposal
update-when: Standard-library schema composition or operation reference support, package-schema eligibility, or its LSP and MCP acceptance evidence changes.
---

# Standard-Library Schema References

## Outcome And Readiness

Let editors and agents find consumer uses of a public standard-library schema
from a saved workspace source. A selection in schema composition, `decode`, or
`encode` returns one reference set for that schema across the selected project.

This is a selectable slice of [Agent Language Services](agent-language-services.md).
The shared package snapshots, standard-library navigation inputs, dependency
schema reference identity, and both adapters already exist. No new syntax,
protocol tool, package API, or plugin is required. Standard-library schema
aliases, broader dependency alias scopes, and umbrella conformance completion
are independent follow-up work.

## Current Boundary

[Editor Support](../specification/editor-support.md) and
[MCP Navigation](../specification/mcp.md#saved-workspace-navigation) currently
admit direct-dependency schema composition and operation references but exclude
standard-library schemas. The language-service tests
`standard_library_schema_composition_is_not_a_dependency_reference` and
`package_schema_references_require_public_exported_direct_dependencies` in
`crates/veln-language-service/src/tests/dependencies_schema_references.rs`
check that exclusion. MCP also checks the empty result with an injected
standard-library schema. These negative expectations must change only for
the eligible standard-library rows below.

The new capability is reference search for a previously excluded package
origin. Increasing path depth, result count, or alias-chain length is not its
purpose. Definition, rename, import semantics, and standard-library exports
remain governed by their current specifications.

## Proposed Contract

An eligible schema is a uniquely resolved, valid-cased public schema in an
exported module of the retained standard-library snapshot. Its identity
includes the package origin, module, and declaration. A same-spelled workspace
or direct-dependency schema is a different identity.

Apply the existing direct-dependency schema composition and operation rules
to this origin: supported leaf roles, exact import precedence, unique implicit
module aliases, duplicate and recovered import rejection, valid repeated
counts, and schema-alias collision blockers retain the same meaning. Imports
must resolve to `std`; a loaded snapshot alone does not grant visibility.

The following table is the planned acceptance model. None of its new positive
standard-library results is claimed as passing evidence yet.

| Input or event | Required observation | Planned evidence |
| --- | --- | --- |
| Import an exported `wire::Packet` from `std`; select its leaf in a direct field, `Repeat` payload, array payload, `decode`, or `encode`. | Every selection returns the same exact union of those resolved leaves in owned project sources, ordered and deduplicated by the current reference contract. | Shared navigation table with hand-authored expected spans; corresponding LSP and MCP adapter cases. |
| Use the full imported module path and its unique implicit leaf alias, including imports shared by sources with the same explicit module. | Both spellings select the same standard-library declaration and union. | Shared navigation import matrix. |
| A workspace schema and a direct-dependency schema have the same module and declaration spelling as the standard schema. | Each selection returns only uses resolving to its own package and declaration identity. | Mixed-origin exact-location cases. |
| An exact import competes with an implicit module alias, or exact imports are conflicting, duplicated, or syntax-recovered. | Exact valid imports retain precedence; ambiguous or invalid imports return no references. Source order does not change the result. | Import matrix in both declaration orders. |
| Select private, non-exported, invalid-cased, recovered, or unresolved schema leaves, or a standard-library schema alias. | Return a successful empty reference set. A schema alias with the same name blocks fallback to a schema, including a recovered alias. | Eligibility and collision table using injected standard-library snapshots. |
| Select an invalid repeated-count payload, module qualifier, import token, alias-target expression, comment, or string. | It does not select an eligible schema reference and does not enter another selection's union. | Leaf-role and lexical-noise table. |
| The same schema is used in multiple owned sources, another selected project, an unselected descendant project, and package implementation sources. | A project request includes only the selected project's owned consumer leaves. Declaration locations and package-source locations are excluded, including when LSP requests declaration inclusion. | Cross-project adapter fixtures and exact URI/range assertions. |
| Request references through LSP and MCP against the same saved inputs. | Normalized locations agree. MCP preserves project scope metadata, checked result schemas, ordering, and continuation behavior. | Paired adapter cases with Unicode coordinate conversion and a paginated MCP result. |
| Change an LSP document overlay. | LSP references reflect the effective overlay; MCP continues to reflect saved files. | Overlay-versus-saved adapter test. |
| Exhaust the existing stable-capture retry bound during an MCP request for an eligible standard schema. | Return `snapshot_changed` without partial references or package publication; preserve selection and previously published resources. | Focused MCP capture-failure test. |

Anonymous and unselected-source requests retain the current single-file scope
contract. This slice does not grant them project-wide package search. Invalid
positions, cursors, and request shapes retain the existing adapter errors.

## Evidence And Completion

Use synthetic standard-library snapshots through the existing language-service
and MCP test injection facilities to verify schema eligibility without adding
an unrelated public schema to the shipped standard library. Adapter evidence
must exercise both LSP and MCP reference handling, not only the shared query.
Use checked cases under `examples/specification/lsp/` and
`examples/specification/mcp/` when the harness can supply the needed snapshot;
otherwise use adapter tests with the injected snapshot and exact expected
protocol results. Record that fixture limitation in the specification evidence
route. The acceptance table owns the planned observations; test output must
not supply its own expected reference set.

Completion requires passing positive and exclusion rows, retained coverage for
direct-dependency schemas and excluded standard-library aliases, and updated
current behavior and evidence routes in `editor-support.md` and `mcp.md` under
`docs/specification/`. Remove this proposal and its Ready entry when those
conditions hold. Keep the umbrella's remaining alias, transitive, recovery,
casing-neutral, declaration-inclusion, conformance, and plugin work separate.
