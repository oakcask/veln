---
role: implementation-record
update-when: The MCP references tool schema, saved package public function-alias navigation boundary, package alias-reference tests, or executable MCP package function-alias case changes.
---

# MCP Saved Package Function-Alias References

The completed slice exposes references to public function aliases from
exported direct-dependency modules and embedded standard-library modules
through the existing MCP `references` tool. Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-package-function-alias` executable MCP specification case
  checks direct-dependency public function-alias references for qualified calls
  and qualified function-value occurrences, workspace `file:` result
  locations, project-wide scope metadata, target-function separation,
  function-alias chain exclusion, and dependency source resource admission.
- `veln-language-service` tests check direct-dependency and standard-library
  public function-alias identity, qualified call and qualified function-value
  references, target-function separation, collision exclusion, private and
  non-exported visibility,
  unsupported type and schema alias selections, alias-chain exclusion, package
  source exclusion, and selected-project source boundaries.
- `veln-mcp` server tests check saved direct-dependency and standard-library
  adapter behavior, collision and visibility exclusions, source isolation,
  retry exhaustion without success-only reference or scope fields, and state
  preservation without partial package resource admission.

Out-of-scope pagination, declaration inclusion, package-source reference
locations, public type-alias references, public schema-alias references,
function-alias chain traversal, recovery symbols, rename behavior, and
transitive-dependency references remain outside the implemented `references`
result.
