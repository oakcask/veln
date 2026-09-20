---
role: specification
authority: normative
update-when: The shared CLI package-root selection, source discovery, dependency loading, manifest source gate, or command analysis pipeline changes.
specification-coverage: usage=#shared-command-analysis; behavior=#shared-command-analysis; limits=#shared-command-analysis
---

# Shared Command Analysis

Commands that analyze source resolve the invocation directory to a package root
before discovery. The nearest ancestor containing a regular `veln.toml` wins.
The marker itself is not followed. A directory, symbolic link, or other
non-regular marker is ignored. If no ancestor qualifies, the invocation
directory becomes an anonymous package root.

A failure while classifying a marker fails the command and does not cause a
wider-ancestor search. Once a root is selected, failure to read its manifest
also fails the command without fallback. Relative source and test arguments
remain relative to the invocation directory. Inputs outside the selected root,
inside a nested package, or reached through a disallowed path fail ownership
validation.

`check`, `run`, `test`, and `repair` share source discovery, generated
doctest creation when requested, parse diagnostics, parse-clean module loading,
semantic diagnostics, checked-core readiness, and selected-entry typed-IR
readiness. Selection, output, execution, and write policy remain command
specific.

## References

The shared selector is implemented in `crates/veln-cli/src/commands/mod.rs`.
Package-root and invocation-relative input behavior is covered by the project
selector and CLI harness tests.
