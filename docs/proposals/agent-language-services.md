---
role: proposal
update-when: The unresolved MCP references, virtual-resource, published-language-reference, conformance, or client-plugin acceptance boundary changes.
---

# Agent Language Services

## Summary

Complete the unresolved agent-facing language-service capabilities after the
implemented MCP workspace inventory, saved diagnostics, saved definitions, and
response-local JSONL harness foundations. Current behavior is specified in
[MCP Workspace Projects, Diagnostics, And Definitions](../specification/mcp.md).
Completed rationale and evidence routes are recorded in
[Agent Language Service Foundations](../reference/implemented-proposals/agent-language-services-foundations.md).

## Unresolved Scope

The proposal retains only these planned capabilities:

- saved workspace function references, followed by the remaining closed
  navigation symbol matrix;
- dependency and standard-library virtual locations and MCP resource reads;
- generated package documentation and the published language reference;
- bounded search, pagination, retained snapshot-resource lifetime, and
  cross-adapter conformance;
- Codex and Claude Code plugin packages and client-native validation.

The proposal does not reopen implemented workspace selection, `check_project`,
or workspace `definition` behavior. Changes to those contracts require a new
bounded proposal or a specification defect fix.

## Language-Semantics Prerequisite

The complete navigation matrix depends on
[Identifier Casing](identifier-casing.md). Adapters must consume the shared
language service's selected symbol identity. They must not add independent
callable-versus-constructor precedence.

## Next Slice: Saved Workspace Function References

The next selectable slice exposes the shared language service's current
workspace-function reference result through `veln mcp`.

| Included | Excluded |
| --- | --- |
| Checked `source`, `line`, and `column` input. | Declaration policy, page size, continuation cursors, and retained cursor state. |
| Selected-project and anonymous single-file capture. | Dependency, standard-library, and virtual-location reference search. |
| Canonical project-owned function reference locations in deterministic order. | New symbol kinds and broader definition coverage. |
| Explicit project or single-file scope metadata. | Changes to name resolution, lowering, LSP behavior, or the shared symbol set. |
| Empty success for a valid position without a supported function search. | Exhaustive callable-expression or shadowing expansion. |
| Existing path, coordinate, schema, and stable-capture failures. | Documentation tools, pagination, resource lifetime, and plugins. |

The shared language-service result is an input to this adapter slice. A defect
outside these rows is separate work unless it prevents one row from passing.

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select a project-owned function call. | Return only that function's project-owned reference sites in deterministic location order, with project scope metadata. | One MCP stdio case containing a declaration, recursive call, ordinary call, and unrelated ambiguous constructor call. |
| Select the unrelated constructor call. | Return an empty reference list. | The same MCP stdio case. |
| Select an accepted source outside the selected project's owned-source set. | Analyze only that source and report `project_wide: false`. | One table-driven descendant-boundary or anonymous-source case. |
| Supply an unaddressable positive coordinate or schema-invalid coordinate. | Return `invalid_position` or protocol invalid params, respectively. | The MCP stdio case and schema tests. |
| Exhaust stable capture after source identity or bytes change. | Return `snapshot_changed` with no partial reference fields. | The shared mutation test, a focused adapter-route test, and an adapter result test. |
| List tools after initialization. | Advertise the checked `references` input and result schemas. | The workspace-lifecycle tool-list case. |

Stable-capture evidence may be composed only when all three named tests exist:
the shared boundary proves unstable input cannot produce a snapshot, the
adapter-route test proves `references` uses that boundary, and the adapter
result test proves `snapshot_changed` with no success-only fields. An adapter
that buffers partial results before capture succeeds needs its own atomic
publication seam.

This slice stops when the six rows pass and its contract is promoted to the MCP
specification and executable examples. New expression forms, symbol kinds, or
LSP navigation cases do not extend this slice automatically.

## Remaining Capability Boundaries

### Navigation Completion

After the bounded function-reference slice, navigation may expand only through
an explicitly revised closed matrix. Each added symbol kind must name its
identity boundary, collision cases, deterministic location ordering, and
adapter-neutral language-service evidence.

### Virtual Locations And Package Documentation

Dependency and standard-library locations use content-addressed virtual URIs,
not materialization paths. MCP resource reads must return the exact captured
bytes used for analysis. Private or excluded resources, malformed URI aliases,
unknown digests, and capacity failures must fail without filesystem fallback.

Generated package documentation must expose only the declared public catalog.
Generation failure must publish no partial catalog. Search and read operations
must use bounded result sizes and deterministic ordering.

### Published Language Reference

The public reference must be a deterministic projection of executable grammar,
checked examples, compiler-owned public tables, and narrowly scoped supporting
prose. It must not contain proposal text, repository maintenance routes, or
physical repository paths.

### Conformance And Plugins

One versioned conformance manifest is the finite completion gate. It must map
every retained requirement and supported client-platform cell to checked
evidence and reject missing, duplicate, skipped, orphaned, or undeclared
mappings. Adding a capability requires a new manifest version or an explicit
compatible extension entry.

Codex and Claude Code plugin packages must bind the active workspace, start a
contract-compatible `veln mcp`, and fail before capability use when the Veln
executable is missing, shadowed, or incompatible. Plugin instructions must
route current behavior to specifications rather than proposal text.

## Final Acceptance Model

| Capability | Completion condition | Planned evidence |
| --- | --- | --- |
| Navigation | Every symbol in the revised closed matrix has definition/reference identity, collision, scope, ordering, and failure coverage. | Shared language-service tables plus LSP and MCP adapter cases. |
| Virtual resources | Every published URI round-trips to retained captured bytes; malformed, private, unknown, and stale requests fail closed. | Resolver matrices and MCP resource lifecycle cases. |
| Package and language documentation | Generation, search, and reads are deterministic, bounded, and atomic. | Freshness checks, failure matrices, and renderer equivalence tests. |
| Conformance | The versioned manifest covers every retained requirement and supported client-platform cell. | Repository conformance validator and injected-failure tests. |
| Plugins | Each supported client completes its native MCP lifecycle with the pinned contract. | Pinned client validation and smoke cases. |

The proposal completes only when all five rows pass and every implemented
contract has moved to specification and executable-evidence routes.

## Non-Goals

- Completion, hover, MCP formatting, or rename edit calculation.
- MCP document overlays or client-supplied in-memory source.
- Remote transport, authentication, or persistent cross-session indexes.
- Registry package versions in virtual URIs.
- Automatic client configuration mutation by `veln`.
