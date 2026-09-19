---
role: proposal
update-when: The remaining package-navigation, cross-adapter-conformance, or client-plugin scope changes.
---

# Agent Language Services

This umbrella records only the unimplemented follow-up work for language
services. The implemented workspace, direct-dependency, and standard-library
navigation behavior is specified by [Editor Support](../specification/editor-support.md)
and [MCP Workspace Projects, Resources, And Navigation](../specification/mcp.md).

## Remaining scope

The following work remains planned:

- standard-library and other dependency schema-alias navigation beyond the
  currently supported direct-dependency boundary;
- transitive-dependency navigation;
- recovery and casing-neutral reference navigation;
- remaining package definition and reference symbol classes;
- cross-adapter conformance evidence for saved navigation;
- Codex and Claude Code plugin packaging and client-native installation flows.

Each follow-up must define its own observable acceptance cases and executable
evidence before implementation. It must update the matching current
specification page and remove its completed scope from this inventory.

## Non-goals

This proposal does not redefine the current `veln lsp` or `veln mcp` protocol,
the saved-workspace capture boundary, standard-library source resources, or
the implemented public standard-library schema composition and operation
references. Those contracts are owned by the current specification pages and
their checked adapter and language-service tests.
