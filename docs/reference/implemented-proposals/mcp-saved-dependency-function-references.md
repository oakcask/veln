---
role: implementation-record
update-when: The MCP references tool schema, saved dependency navigation, direct-dependency function-reference boundary, public function-alias target separation, or executable MCP dependency-reference cases change.
---

# MCP Saved Dependency Function References

The completed slice exposes references to visible direct-dependency functions
from a selected saved workspace project through the existing MCP `references`
tool. Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-dependency-function` executable MCP specification case checks
  qualified call and qualified function-value occurrences, workspace `file:`
  result locations, project-wide scope metadata, unsupported import-alias
  selection, dependency declaration exclusion, dependency body exclusion, and
  same-session dependency source resource admission.
- `veln-language-service` tests check direct-dependency function selection and
  reference collection for qualified calls, qualified function-value
  occurrences, package identity boundaries, module identity boundaries, public
  function-alias target separation, local binding and field exclusion, and
  selected-project source boundaries.
- `veln-mcp` server tests check saved-project inference, path, vendor, mirror,
  and local git source forms, import-alias selection, identity collisions,
  public function-alias boundaries, unsupported selections, dependency-source
  capture retry exhaustion, retained resource-capacity failure, and state
  preservation without partial reference results.

Out-of-scope agent language-service work remains planned only when a separate
Ready proposal selects it from [../../proposals/README.md](../../proposals/README.md).
