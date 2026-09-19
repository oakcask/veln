---
role: proposal
update-when: MCP reference input, declaration-location policy, reference pagination, package virtual locations, or declaration-inclusion evidence changes.
---

# MCP Reference Declaration Inclusion

## Outcome And Readiness

Let an MCP client explicitly request the selected symbol's declaration together
with its saved-source references. This is a selectable slice of
[Agent Language Services](agent-language-services.md).

The shared navigation result already separates one declaration location from
the workspace reference spans. LSP already applies an explicit declaration
policy. MCP already has checked reference schemas, stable saved-project
capture, package virtual locations, deterministic ordering, and pagination.
This slice therefore requires no new language syntax, symbol class, workspace
scope, package capture, or tool.

## Current Boundary

[MCP Workspace Projects, Resources, And Navigation](../specification/mcp.md#saved-workspace-navigation)
currently defines an initial `references` request with `source`, `line`,
`column`, and optional `page_size`. MCP always excludes the declaration. The
focused test `references_exclude_declarations_for_new_workspace_symbol_classes`
in `crates/veln-mcp/src/server/tests/references/scope_and_symbol_boundaries.rs`
checks that boundary.

[Editor Support](../specification/editor-support.md) already makes LSP
declaration inclusion conditional for workspace declarations and excludes
package declarations. The MCP option proposed here is adapter policy over the
same shared navigation result. It does not change LSP behavior.

## Proposed Contract

Add optional boolean `include_declaration` to the initial MCP `references`
input. Omission is equivalent to `false`, preserving the current result for
existing requests. A continuation request still contains only `cursor`; the
captured first request fixes its declaration policy for every page.

When `include_declaration` is `true`, add exactly the selected symbol's eligible
declaration location to the complete result before the existing deterministic
sort and pagination are applied. Do not add import sites, alias targets,
package implementation uses, or another declaration with the same spelling.
The existing symbol eligibility, selected-project scope, stable-capture,
ordering, deduplication, and cursor rules continue to apply.

The following table is the planned acceptance model. It does not claim that
the new MCP input or positive results are implemented.

| Input or event | Required observation | Planned evidence |
| --- | --- | --- |
| Omit `include_declaration` or set it to `false` for a supported workspace symbol. | Return the current reference-only set. Omission and explicit `false` are byte-equivalent after ordinary response framing. | Checked schema cases and a table-driven MCP adapter case. |
| Set `include_declaration` to `true` for a supported workspace schema, function, type, constructor, value binding, handler context parameter, or handler operation clause parameter. | Add that symbol's one `file:` declaration location and retain the same reference locations. | Shared-result and MCP exact-location matrix covering every supported workspace symbol class. |
| Select an eligible public direct-dependency or standard-library declaration or public alias through a supported consumer use. | Add its one canonical `veln-pkg:` declaration location. Keep consumer references as `file:` locations and exclude every package-source implementation use. | Dependency and standard-library MCP cases with exact virtual and workspace URIs and a package-resource round trip. |
| Select an unsupported, private, ambiguous, recovered, invalid-cased, or otherwise ineligible symbol. | Preserve the current successful empty result or unsupported boundary. Declaration inclusion does not make the symbol eligible. | Negative symbol and package-visibility matrix. |
| The declaration and references span more than one page. | Sort the combined set by the current location order, page it without gaps or duplicates, and bind the declaration policy to the retained cursor state. A continuation with any field besides `cursor` remains invalid. | MCP pagination transition case with the declaration on a later sorted page, cursor replay rejection, and concatenated-result assertion. |
| Supply a non-boolean or `null` declaration option, add it to a continuation, or combine it with an otherwise invalid initial input. | Reject the request as protocol-invalid without consuming cursor state or changing workspace, resource, or prior-result state. | Checked input-schema rejection cases and same-cursor recovery case. |
| Compare LSP and MCP over the same saved workspace input with declaration inclusion enabled. | After converting coordinates and package location policy, workspace declaration and reference identities match. LSP continues to exclude package declarations; that adapter difference is explicit rather than treated as a semantic mismatch. | Paired LSP and MCP exact-location case over non-BMP input. |
| Stable capture fails, a refresh invalidates a cursor, or package resource capacity prevents publication. | Preserve the existing domain failure and state-preservation behavior. Do not return a partial declaration or reference page. | Focused MCP failure-transition cases. |

Anonymous single-file requests retain their current scope and may include only
a declaration from that captured file. The option does not make their result
project-wide. A dependency initialized as a workspace project continues to use
its workspace `file:` identity rather than a package virtual identity.

## Evidence And Completion

Update the checked MCP input schema first. Use a table-driven adapter matrix
for declaration policy and exact locations, plus focused pagination and
failure-transition cases. Pair one saved non-BMP source shape with LSP evidence
so coordinate conversion and the intentional package-declaration difference
are reviewable. The shared language-service result remains transport-neutral;
adapter tests must prove URI conversion and pagination rather than duplicating
symbol lookup rules in a second expected-value source.

Completion requires every acceptance row to pass and the current declaration
policy, schema field, cursor binding, package-location boundary, and evidence
routes to be stated in `editor-support.md` and `mcp.md` under
`docs/specification/`. Remove this proposal and its Ready entry after those
conditions hold. Keep broader symbol coverage, transitive dependency,
recovery, casing-neutral navigation, full cross-adapter conformance, published
reference, and plugin work in the umbrella.
