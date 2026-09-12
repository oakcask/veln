---
role: implementation-record
update-when: The MCP references tool schema, saved package public function-alias navigation boundary, package alias-reference tests, standard-library alias-reference tests, or executable MCP package function-alias case changes.
---

# MCP Saved Package Function-Alias References

The completed slice exposes references to public function aliases from exported
direct-dependency modules and exported embedded standard-library modules
through the existing MCP `references` tool. Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-package-function-alias` executable MCP specification case
  checks direct-dependency public function-alias references for qualified calls
  and qualified function-value occurrences, standard-library public
  function-alias references for accepted bare implicit-prelude and
  `prelude::`-qualified occurrences, workspace `file:` result locations,
  project-wide scope metadata, target-function separation, qualified alias
  exclusion in type annotations and type arguments, function-alias chain
  exclusion through a non-exported package module, and dependency source
  resource admission.
- `veln-language-service` tests check direct-dependency and standard-library
  public function-alias identity, qualified call and qualified function-value
  references, direct-dependency import-alias qualification, accepted bare
  implicit-prelude forms, target-function separation, value-namespace
  collision exclusion, private and non-exported visibility, invalid-cased alias
  exclusion, unsupported type and schema alias selections, same-module and
  non-exported-module alias-chain exclusion, package source exclusion, and
  selected-project source boundaries.
- `veln-mcp` server tests check saved direct-dependency and standard-library
  adapter behavior, collision and visibility exclusions, standard-library
  function-alias chain empty results, source isolation, anonymous and
  descendant-manifest single-file scope for package alias selections outside
  the selected project, retry exhaustion without success-only reference or
  scope fields for dependency and standard-library alias selections, and state
  preservation without partial package resource admission.

Out-of-scope pagination, declaration inclusion, package-source reference
locations, public type-alias references, public schema-alias references,
function-alias chain traversal, recovery symbols, rename behavior, and
transitive-dependency references remain outside the implemented `references`
result.
