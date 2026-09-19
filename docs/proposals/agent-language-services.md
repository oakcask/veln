---
role: proposal
update-when: The remaining agent-language-service navigation, conformance, or plugin work changes.
---

# Agent Language Services

This proposal is an inventory for the remaining `veln mcp` work. The shared
saved-source navigation foundation and the bounded workspace, direct-
dependency, and standard-library schema reference slices are current behavior
specified under `docs/specification/`; they are not planned work here.

## Remaining work

- Extend package navigation to standard-library schema aliases and other
  dependency schema-alias scopes.
- Define and implement transitive-dependency navigation.
- Define recovery and casing-neutral reference behavior.
- Decide whether declaration locations are included in each remaining
  reference result and add the corresponding contract evidence.
- Complete cross-adapter conformance evidence for saved navigation.
- Package and validate the Codex and Claude Code plugins.

The remaining work must update the matching current specification and checked
evidence when its behavior becomes implemented. This page is not a source of
current language behavior and is not itself selectable until a narrower Ready
proposal defines one of these items.
