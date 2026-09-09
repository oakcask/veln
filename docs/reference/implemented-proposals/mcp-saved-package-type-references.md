---
role: implementation-record
update-when: The MCP references tool schema, saved package type navigation boundary, package type-reference behavior, or executable MCP package-type-reference cases change.
---

# MCP Saved Package Type References

The completed slice exposes references to visible direct-dependency and
embedded standard-library type declarations from a selected saved workspace
project through the existing MCP `references` tool. Current behavior is
specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-package-type` executable MCP specification case checks
  direct-dependency type references, embedded standard-library prelude type
  references, type annotation and return type occurrences, constructor
  qualifier type segments, workspace `file:` result locations, project-wide
  scope metadata, import-alias segment exclusion, workspace type collisions,
  other-package collisions, fields, strings, and comments.
- `veln-language-service` tests check direct-dependency type selection and
  reference collection across qualified type roles, aliases, multiple project
  sources, constructor qualifier type segments, package identity collisions,
  workspace type collisions, other-package type collisions, fields, strings,
  comments, private package types, non-exported package modules, public type
  aliases, invalid-casing package types, explicit standard-library module
  types, and implicit prelude type references.
- `veln-mcp` server tests check the saved-project adapter boundary for
  direct-dependency and standard-library package type references, project-wide
  scope metadata, workspace-only result locations, collision filtering,
  unsupported package type selections, and `snapshot_changed` retry exhaustion
  without success-only fields or partial dependency resource admission.

Out-of-scope pagination, declaration inclusion, package-source reference
locations, constructor-symbol references, schema references, public type-alias
references, recovery symbols, and transitive-dependency references remain
outside the implemented `references` result.
