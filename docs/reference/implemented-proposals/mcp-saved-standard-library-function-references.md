---
role: implementation-record
update-when: The MCP references tool schema, saved standard-library navigation boundary, standard-library function-reference behavior, or executable MCP standard-library-reference cases change.
---

# MCP Saved Standard-Library Function References

The completed slice exposes references to public function declarations from
exported embedded standard-library modules through the existing MCP
`references` tool. Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-standard-library-function` executable MCP specification case
  checks accepted bare implicit prelude calls, qualified calls, qualified
  function-value occurrences, workspace `file:` result locations, and
  project-wide scope metadata.
- `veln-language-service` tests check explicit standard-library module
  function references, implicit prelude function references, qualified
  function-value references, package-origin identity, lexical shadowing,
  private and non-exported visibility, public function-alias exclusion, and
  invalid-casing rejection.
- `veln-mcp` server tests check the saved-project adapter boundary for
  standard-library public function references and keep package public
  function-alias selections empty.

Out-of-scope pagination, declaration inclusion, package-source reference
locations, public function-alias references, non-function package symbols,
recovery symbols, and transitive-dependency references remain outside the
implemented `references` result.
