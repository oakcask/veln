---
role: proposal
update-when: Shared recovery navigation, MCP reference filtering, MCP pagination, saved-navigation conformance, or planned recovery-reference evidence changes.
---

# MCP Recovery References

## Outcome And Readiness

Expose the shared recovery-reference set through the MCP `references` tool.
This is a selectable slice of
[Agent Language Services](agent-language-services.md).

The shared language service already selects a unique class-compatible recovery
identity, returns its retained declaration, and collects linked in-scope
references. LSP definition, references, prepare-rename, and rename already use
that result. MCP definition and rename also support the same recovery identity,
but MCP `references` deliberately filters every recovery result after shared
navigation succeeds. Removing that adapter-only exclusion and adding MCP
evidence is the remaining bounded gap.

The MCP adapter already provides saved-source capture, Unicode-scalar
coordinates, declaration inclusion, deterministic ordering, pagination,
cursor authentication, and state-preserving failures. This slice needs no new
parser, semantic rule, shared symbol classifier, package capture, request
field, or protocol result shape.

## Scope

The slice covers the existing unique recovery identities for invalid-cased
workspace types, constructors, functions, tests, function parameters, result
bindings, local and pattern bindings, `satisfy` candidate bindings, handler
context parameters, and handler operation-clause parameters. It uses the
declaration and linked-reference set returned by the shared language service
without reclassifying source in the MCP adapter.

Normal workspace and package symbols keep their current behavior. Recovery
identity creation, class compatibility, lexical scope, shadowing, ambiguity,
qualified-occurrence exclusion, rename, diagnostics, and LSP behavior are
outside this slice and remain unchanged.

## Proposed Contract

When shared navigation returns one eligible recovery identity, MCP
`references` returns that identity's complete linked workspace reference set.
Selecting the retained invalid declaration or any linked reference produces
the same set. Each location uses the saved workspace `file:` URI and the
existing one-based Unicode-scalar half-open range.

`include_declaration: false` excludes the retained invalid declaration.
`include_declaration: true` adds it once before the existing sort and paging
steps. The result continues to use the request's current project or anonymous
single-file scope. No recovery location admits or publishes a package
resource.

## Acceptance Model

| Input or boundary | Observable result | Planned evidence |
| --- | --- | --- |
| Select the retained invalid declaration or any linked reference for each existing recovery symbol class. | Return the same complete linked-reference set for every selection. Preserve the shared identity, lexical scope, and reference ranges. | Table-driven MCP server cases that cover type, constructor, function or test, function parameter, result binding, local or pattern binding, `satisfy` candidate, handler context parameter, and handler operation-clause parameter recovery. |
| Request references with `include_declaration: false`. | Return linked references without the retained invalid declaration. A recovery identity with no linked references succeeds with an empty list. | Exact MCP result cases for a referenced identity and a declaration-only identity. |
| Request references with `include_declaration: true`. | Add the retained invalid declaration exactly once. Sort and page the combined locations by the existing MCP rules. | Exact LF, CRLF, and non-BMP coordinate cases plus a page boundary separating the declaration and linked references. |
| Recovery candidates are ambiguous, class-incompatible, shadowed, qualified, inside a local-binding initializer, or outside the retained lexical scope. | Return the current successful empty result for an unsupported selection and exclude unrelated or text-only occurrences from a valid identity's result. | Negative MCP cases aligned with the shared recovery selection and linking matrix. |
| The selected invalid source has an invalid module identity, or the selection is a package record or a symbol class without an existing recovery identity. | Preserve the current empty or unsupported result. Do not reinterpret it as a workspace recovery identity. | Source-identity, package-origin, and unsupported-class boundary cases. |
| A saved capture, position, path, continuation cursor, refresh, or resource-capacity operation fails. | Preserve the existing failure and state. Return no partial recovery result and do not create, consume, or revive an unrelated cursor or resource. | Existing MCP transition harness extended with a recovery selection before and after the failure. |
| Run the same recovery selection through LSP and MCP over one unchanged saved workspace. | After coordinate and declaration-policy normalization, both adapters return the same shared recovery definition and references. | Recovery rows in the saved-navigation cross-adapter conformance harness. |

## Verification And Completion

Keep recovery selection and reference collection in `veln-language-service`.
MCP must consume that result without rebuilding recovery scope or
class-compatibility rules. MCP server tests and a checked MCP transcript must
verify serialization, declaration policy, coordinates, pagination, and
failure preservation. The saved-navigation conformance harness must establish
that both adapters expose the same shared result.

Completion requires every acceptance row to pass. Update the recovery
reference behavior and limits in `mcp.md`, then remove this page and its Ready
catalog entry. The current editor specification already owns shared and LSP
recovery navigation and needs an update only if implementation changes that
contract. Keep transitive-dependency navigation, remaining package symbols,
imported effects and handlers, agent integration, the conformance gate, and
client packaging in the umbrella until each has its own complete contract.
