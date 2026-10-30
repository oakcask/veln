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

During shared semantic analysis, every parse-clean module immutable value is
checked against its written annotation and the finite-data initializer rules.
Same-module and eligible imported references are resolved before function
bodies are checked. Imported selection retains package-aware declaration
identity and visibility, including public annotation boundaries. Invalid
declarations and their transitive dependents across package sources are
withheld from later semantic facts. When imported availability changes, that
unavailability propagates only through module-value initializers that depend
on the changed declaration; unrelated declarations retain their semantic
facts. This propagation preserves same-module function and function-alias
collisions. Imported bare selection considers only functions and aliases that
remain publicly available, so a signature-visibility failure cannot hide a
valid module value. The same boundary applies when analysis reuses a prepared
standard-library environment.
Initializer or package-wide dependency failures are reported together with
`module.value_not_executable` for each declaration. The latter error keeps the
selection from becoming execution-ready or producing backend output. Accepted
declarations and references remain represented in checked core and typed IR.
Parsing and formatting remain available as specified by the
[source-surface boundary](source-surface.md#module-immutable-value-declarations).

A source with a non-recoverable parse diagnostic is excluded from semantic
analysis. Import-declaration diagnostics recovered at the declaration newline
and ASCII-lowercase final-segment diagnostics for structurally complete variant
refinements permit semantic analysis only when every parse diagnostic in that
source is one of those forms. A recovered import retains diagnostic provenance
but supplies no lookup binding. The
[import source-surface contract](source-surface.md#usage-and-declaration-forms)
and [variant-refinement contract](source-surface.md#variant-refinement-shaped-type-text)
define those recovery boundaries. Other eligible sources in the same project
can still proceed to semantic analysis.

Shared semantic analysis keeps unambiguous type and constructor identities for
recovery. It rejects an inaccessible named type in a public module value
annotation. It also rejects an inaccessible variant refinement in a public
module value annotation, public function signature, or public ADT payload.
Publication quarantine for functions and ADTs propagates
through public signature dependencies, including transparent type aliases and
recursive ADT dependency cycles. A dependent ordinary named-type reference
does not duplicate the causal privacy diagnostic. The pipeline also
quarantines every type exposed by a function, effect operation, handler, or ADT
constructor whose signature contains a refinement union with different
resolved ADT identities or canonical generic arguments. The affected
structural positions, declaration boundaries, and publication closure are
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
