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
doctest creation when requested, parse diagnostics, module loading, semantic
diagnostics, checked-core readiness, and selected-entry typed-IR readiness.
Selection, output, execution, and write policy remain command specific.

A source with parse diagnostics is excluded from semantic analysis. Other
parse-clean sources in the same project can still proceed to semantic
analysis.

Shared semantic analysis keeps unambiguous type and constructor identities for
recovery, but it does not publish a public function or ADT whose signature
exposes an inaccessible variant refinement. Publication quarantine propagates
through public signature dependencies, including transparent type aliases and
recursive ADT dependency cycles. A dependent ordinary named-type reference
does not duplicate the causal privacy diagnostic. The pipeline also
quarantines every type exposed by a function, effect operation, handler, or ADT
constructor whose signature contains a refinement union with different
resolved ADT identities or canonical generic arguments. The pipeline also
quarantines a declaration whose uppercase refinement final is unknown,
resolves to a non-constructor, or names a constructor owned by another ADT.
That recovery retains
the base identity for independent analysis but does not publish a constructor
identity. The affected structural positions, declaration boundaries, and
publication closure are
defined by the
[type specification](types.md#compatibility-and-limits); diagnostic order,
spans, and related context are defined by the
[diagnostic specification](diagnostics-json.md#diagnostic-families).

`doc` consumes the same project semantic diagnostics for its selected production
sources, but retains its own source exclusion, parse recovery, schema-reference,
and write boundaries. A refinement diagnostic remains an error when recovery
retains an identity; recovery does not turn the invalid annotation into a
successful static transition.

When doctest creation is requested, a documented source path that is not a
canonical package-relative virtual-source origin produces
`module.invalid_source_path` against the original source. The shared pipeline
continues discovery and analysis for other selected sources. The origin and
generator-identity boundary is specified by
[Call-site Declarations](call-site-declarations.md#declaration-behavior).

## References

The shared selector is implemented in `crates/veln-cli/src/commands/mod.rs`.
Package-root and invocation-relative input behavior is covered by the project
selector and CLI harness tests.
