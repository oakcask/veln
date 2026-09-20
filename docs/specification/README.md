---
role: routing
update-when: A current language specification topic or its documentation route changes.
---

# Language Specification

This directory routes implemented Veln behavior. Read the smallest topic page
that answers the question. Each behavior page explains usage, rules, and limits;
its references provide supporting implementation and test evidence.

## Read first

- [overview.md](overview.md) defines the stability boundary.
- [topic-map.md](topic-map.md) selects a subject page.
- [../reference/documentation-authoring.md](../reference/documentation-authoring.md)
  defines maintenance and verification rules.

## Routes

- Source grammar, names, types, effects, contracts, and holes:
  [topic-map.md#source-surface](topic-map.md#source-surface).
- Commands and machine-readable output:
  [topic-map.md#commands-and-output](topic-map.md#commands-and-output).
- Runtime, networking, and examples:
  [topic-map.md#runtime-examples-and-rationale](topic-map.md#runtime-examples-and-rationale).
- Editor and LSP behavior: [editor-support.md](editor-support.md).
- MCP resources and tools: [mcp.md](mcp.md).
- Package snapshots, documentation, and virtual sources:
  [package-snapshots.md](package-snapshots.md),
  [package-documentation.md](package-documentation.md), and
  [package-virtual-sources.md](package-virtual-sources.md).
- Generated reference artifact: [language-reference-catalog.md](language-reference-catalog.md).
- Rationale: [source-decisions.md](source-decisions.md).

Proposal pages describe unfinished work and are not current behavior.
