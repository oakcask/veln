---
role: specification
authority: normative
update-when: The veln check command gate, source discovery behavior, dependency loading, or output contract changes.
specification-coverage: usage=#check-command; behavior=#discovery-and-ownership; limits=#limits-and-errors
---

# Check Command

Use `veln check [PATH ...]` to analyze source. With no paths, it recursively
selects owned regular `.veln` files below the selected package root. A
directory path is searched recursively; a file path selects that file. The
final list is sorted and deduplicated. With `--json`, the command emits the
check diagnostic envelope; otherwise it prints diagnostics or `ok`.

## Discovery and ownership

Discovery does not follow source or directory symbolic links and skips `.git`
directories. A regular descendant `veln.toml` starts a nested package and
excludes that directory and descendants from outer discovery. A symlink or
non-regular object named `veln.toml` is not a boundary. `target` has no
special source meaning.

Explicit files and directories must remain under the selected root, must not
cross a nested package boundary, and must not escape through parent paths or
symbolic links. One invalid input fails the complete discovery operation.
Boundary-classification errors also fail; the command does not silently omit
the path.

## Manifest and dependencies

When a manifest is present, `check` validates package/tool metadata,
dependencies, and `[lib].exports`. Path, vendor, mirror, and already
materialized git dependencies may be loaded for imports. Git entries require a
remote and exactly one of `rev`, `tag`, or `branch`; `subdir` selects a
package root within the repository. Source analysis does not fetch, clone,
resolve revisions, update checksums, or write a lockfile.

An export is a package-relative `.veln` file path, not a module path. It must
match a selected source, derive a valid module identity, avoid test companions,
and not duplicate a derived module. `[modules]` is rejected. An imported
dependency must have a direct regular `veln.toml`, matching package name, and
an exported imported module; only public declarations and aliases cross that
boundary.

## Analysis gates

A parse-clean module immutable value declaration produces
`module.value_not_executable`, so `check` does not report success for a source
that contains one. Before applying this checked-core gate, analysis validates
every initializer against its written annotation, resolves acyclic same-module
references including forward references, resolves eligible imported references
with package-aware identity and visibility, and reports unsupported initializer
forms and same-module cyclic components. Public annotations cannot expose
private types or variant refinements. Invalid declarations and their dependents
within the same-module dependency graph do not provide later semantic facts.
Imported references in another module value initializer use each imported
declaration's final availability. Bare and qualified initializer reads
therefore use the same final import exposure in every source and import order.
When an imported declaration becomes unavailable, importers are rechecked and
that unavailability can continue through later imported initializer reads.
Cross-module references do not yet create dependency-graph edges or
cross-module cycle diagnostics. This gate still prevents lowering or
publication. The
[source-surface specification](source-surface.md#module-immutable-value-declarations)
defines the declaration boundary, and the
[diagnostic specification](diagnostics-json.md#diagnostic-families) defines
the human and JSON record.
An available module value used as a public function-alias target also fails
analysis with `name.kind_mismatch`; an unavailable target retains
`name.unresolved`. The [name-resolution specification](name-resolution.md#namespaces-and-use-roles)
defines target selection, and the diagnostic specification defines the human
and JSON fields.

Parse diagnostics suppress semantic diagnostics for that file unless every
parse diagnostic is one of these recoverable forms:

- An import-declaration diagnostic recovered at the declaration newline.
  Analysis retains the written target, an identifier-shaped alias, and a
  parseable package source for independent casing and target diagnostics. The
  recovered import supplies no lookup binding and does not participate in
  import collision checks.
- The ASCII-lowercase final-segment error for a structurally complete variant
  refinement. Analysis reports independently provable semantic diagnostics
  without accepting the annotation.

For either form, `check` emits parse diagnostics before semantic diagnostics.
A parse-clean non-generic path with lowercase base and final segments follows
the same output contract when unique initial-only base recovery selects its
refinement interpretation; `check` inserts the parse diagnostic before casing
and type diagnostics. Any other parse diagnostic restores the file-level
suppression. Other eligible files may still report semantic errors.
The [import source-surface contract](source-surface.md#usage-and-declaration-forms)
and [variant-refinement contract](source-surface.md#variant-refinement-shaped-type-text)
define the two recoverable syntax boundaries. The
[diagnostic specification](diagnostics-json.md#diagnostic-families) defines
the overlap and ordering. The checked
[`explicit-import-alias-combined-order-human`](../../examples/specification/check/explicit-import-alias-combined-order-human/)
and
[`explicit-import-alias-combined-order-json`](../../examples/specification/check/explicit-import-alias-combined-order-json/)
cases fix the import-recovery output order. Cross-file imports and qualified
calls from analyzed files participate in the same analysis path as `run` and
`test`.
Lowering reports checked-core blockers such as missing expressions and call or
constructor arity mismatches. Parse-clean, semantically valid `begin` and
`defer` forms pass checked-core readiness.
A private variant refinement anywhere in a public-function parameter or
result, or in a public variant payload of a public source ADT, is a semantic
error, so `check` emits no success report. This includes refinements nested in
named type arguments, function parameters and results, record fields, and refinement
unions. Publication quarantine closes over ordinary named ADT dependencies in
public payloads and over quarantined ADTs in public-function parameters and
results. Those ordinary references do not duplicate the causal privacy
diagnostic, but they remain checked-core and typed-IR blockers. The
[type rules](types.md#compatibility-and-limits) define the
publication and checked-output gate. The
[diagnostic contract](diagnostics-json.md#diagnostic-families) defines human
spans, related context, and JSON `exposure_path` segments.
A refinement union whose successfully instantiated alternatives have different
resolved ADT identities or canonical generic arguments is also a semantic
error. It blocks command success and quarantines the complete containing
function, effect operation, handler, or ADT-constructor signature according to
the same type and diagnostic contracts.
A `format binary` schema field whose generated decoder cannot establish a
resolved finite ADT refinement also fails the semantic gate. The
[execution specification](execution.md#schema-execution) defines the decoder
capability boundary, and the
[diagnostic specification](diagnostics-json.md#diagnostic-families) defines
the human and JSON output.
An unresolved, wrong-kind, ambiguous, private, binary, or otherwise ineligible
target in a parse-clean `validate Schema from value at offset` expression also
fails the semantic gate. Human and JSON modes report the target-specific
`schema.validate_expression` contract defined by the diagnostic specification.
The checked
[`schema-runtime-validation-targets-human`](../../examples/specification/check/schema-runtime-validation-targets-human/)
and
[`schema-runtime-validation-targets-json`](../../examples/specification/check/schema-runtime-validation-targets-json/)
cases fix both command projections.
The checked
[`publication-closure`](../../examples/specification/check/adt-variant-refinement-publication-closure-human/)
case verifies that one causal diagnostic blocks command success. Focused
semantic publication tests in
`crates/veln-sema/src/tests/variant_refinement_annotations_and_private_results/public_visibility.rs`
and its `publication_and_recovery.rs` support verify that the ordinary
dependent declarations remain unpublished.
A selected documentation source that cannot form a canonical virtual doctest
path reports `module.invalid_source_path` against the original source. Analysis
continues for other selected sources, and JSON output includes the rejected
origin once alongside their diagnostics. The virtual-source boundary is
specified by
[Call-site Declarations](call-site-declarations.md#declaration-behavior).

## Limits and errors

Discovery is atomic for the invocation: a rejected input produces no partial
success report. Dependency sources must already be available; the command does
not perform dependency acquisition.

## References

Implementation: `crates/veln-cli/src/commands/check.rs`. Discovery and
dependency boundary coverage is under `examples/specification/check/` and
`crates/veln-cli/tests/check_json/check.rs`.
