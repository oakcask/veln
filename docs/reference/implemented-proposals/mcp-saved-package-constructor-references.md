---
role: implementation-record
update-when: The MCP references tool schema, saved package constructor-reference boundary, package constructor navigation behavior, or executable MCP package-constructor-reference cases change.
---

# MCP Saved Package Constructor References

The completed slice exposes references to visible public constructors of
public types from exported direct-dependency and embedded standard-library
modules through the existing MCP `references` tool. Current behavior is
specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-package-constructor` executable MCP specification case
  checks a direct-dependency constructor in a saved selected project,
  workspace `file:` result locations, project-wide scope metadata, canonical
  ordering across project sources, qualified calls, constructor patterns,
  accepted bare forms, package body exclusion, dependency source resource
  admission, workspace collision exclusion, unsupported import-alias segment
  selection, ambiguous module-qualified constructor-leaf empty success, and
  alias-qualified constructor definition selection without reference expansion.
- `veln-language-service` tests check direct-dependency constructor references
  across project sources, package identity boundaries, explicit type-qualified
  constructor calls, module-qualified constructor calls, constructor patterns,
  standard-library prelude constructor references, package-origin identity,
  module-qualified constructor-leaf ambiguity, type-qualified constructor
  disambiguation, alias-qualified constructor definition selection,
  other-package and workspace collision exclusion, public alias route
  reference exclusion, private constructor visibility, and package source
  exclusion.
- `veln-mcp` server tests check saved-project adapter support for
  direct-dependency and standard-library public constructor references,
  workspace `file:` location results, project-wide scope, package source
  exclusion, collision filtering, ambiguous constructor-leaf empty success,
  alias-qualified constructor definition selection with empty references,
  private constructor, import-alias segment, anonymous source, recovery,
  schema, effect, handler, and effect-operation empty-result boundaries, and
  `snapshot_changed` retry exhaustion without success-only fields or partial
  package resource admission.

Out-of-scope pagination, declaration inclusion, package-source reference
locations, type-reference widening, schema references, public-alias
references, recovery symbols, transitive-dependency references, rename
behavior, and package source-location fields remain outside the implemented
`references` result.
