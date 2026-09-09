---
role: implementation-record
update-when: The MCP references tool schema, saved package type-reference boundary, package type navigation behavior, or executable MCP package-type-reference cases change.
---

# MCP Saved Package Type References

The completed slice exposes references to visible public type declarations
from exported direct-dependency and embedded standard-library modules through
the existing MCP `references` tool. Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-package-type` executable MCP specification case checks a
  direct-dependency type and an exported standard-library type in the same
  saved selected project, workspace `file:` result locations, project-wide
  scope metadata, canonical ordering across project sources, type
  annotations, type arguments, return types, type-alias right-hand sides,
  constructor qualifier type segments, package body exclusion, dependency
  source resource admission, constructor-name segment, field, string, comment,
  and lexical-binding collision exclusion, and unsupported import-alias segment
  selection.
- `veln-language-service` tests check direct-dependency type references across
  project sources, package identity boundaries, module identity boundaries,
  workspace, other-package, same-package different-module, and constructor
  spelling collision exclusion, constructor qualifier type segments,
  explicit standard-library type references,
  implicit standard-library prelude type references, package-origin identity,
  private and non-exported visibility, public type-alias selection exclusion,
  and invalid-casing rejection.
- `veln-mcp` server tests check saved-project adapter support for
  direct-dependency and standard-library public type references, constructor
  qualifier selection, workspace `file:` location results, project-wide scope,
  package source exclusion, collision filtering, private type, non-exported
  module, invalid-casing type, public type-alias, package constructor-symbol
  selection, package module-segment, anonymous source, recovery, schema,
  effect, handler, and effect-operation empty-result boundaries, and
  `snapshot_changed` retry exhaustion without success-only fields or partial
  package resource admission.

Out-of-scope pagination, declaration inclusion, package-source reference
locations, constructor-symbol references, schema references, public-alias
references, recovery symbols, transitive-dependency references, and package
source-location fields remain outside the implemented `references` result.
