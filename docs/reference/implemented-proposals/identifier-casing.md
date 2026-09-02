---
role: implementation-record
authority: supporting
update-when: The implemented identifier-casing specification, focused completion records, or remaining dependent casing proposals change.
---

# Identifier Casing

## Outcome

The source identifier-casing foundation is implemented. Type and constructor
roles require an ASCII uppercase initial. Module, function, and value-binding
roles require an ASCII lowercase initial at the implemented declaration,
binding, path, module-identity, and rename boundaries.

Current behavior is specified by:

- [Names And Effects](../../specification/names-effects.md) for declarations,
  bindings, recovery records, and selected command boundaries;
- [Name Resolution](../../specification/name-resolution.md) for import paths,
  qualified uses, and source-path-derived module identities;
- [Types](../../specification/types.md) for constructor forms and patterns;
- [Editor Support](../../specification/editor-support.md) for navigation and
  LSP rename behavior; and
- [MCP Workspace Projects, Diagnostics, And Definitions](../../specification/mcp.md)
  for the implemented MCP definition boundary.

The focused identifier-casing records listed in this directory preserve the
implementation history and executable-evidence routes.

## Retired Umbrella Boundary

The original umbrella proposal mixed completed language behavior with three
independent follow-up surfaces. It is no longer an implementation target.
The remaining or completed follow-up surfaces are routed by separate pages:

- [Repair Candidate Isolation](identifier-casing-repair-candidate-isolation.md)
  records the completed repair tolerant-consumer boundary.
- [Explicit Import Alias Casing](../../proposals/identifier-casing-explicit-import-aliases.md)
  depends on an explicit import-alias syntax and lookup contract.
- [MCP Rename Casing Mapping](../../proposals/identifier-casing-mcp-rename.md)
  depends on an MCP rename tool contract.

Broad requirements about every remaining consumer, boundary, diagnostic
overlap, or source carrier were not retained as proposal targets. A focused
proposal must name a finite surface and executable acceptance evidence. Range,
overlap, and migration cases belong to the focused surface that introduces or
changes the corresponding behavior.

## Completion Boundary

The implemented casing foundation is sufficient for existing shared
language-service function reference selection. Future explicit import aliases,
MCP rename transport behavior, and repair candidate isolation do not change
the accepted-source function reference semantics.
