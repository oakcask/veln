---
role: specification
authority: normative
update-when: The veln doc command selection, generated documentation output, manifest metadata, source-analysis gate, or write gate changes.
specification-coverage: usage=#doc-command; behavior=#metadata-and-generated-content; limits=#limits
---

# Doc Command

Use `veln doc [PATH ...]` to generate deterministic Markdown. Path selection
follows `check`: no paths discovers recursively, directories are recursive,
files are explicit, and the selected paths are sorted and deduplicated.
Exact `.test.veln` companions are excluded from the public document,
including when explicitly named. `_test.veln` integration modules remain
ordinary sources. If no public source remains, metadata is still emitted and
the module section says that no source modules were selected.

## Metadata and generated content

The command accepts string-valued `[package]` and `[tool.<name>]` fields.
Package fields become package metadata; tool fields become tool metadata. A
package `name` becomes the title, otherwise the title is `Veln Project`.

For each selected parse-clean non-companion source, output includes the
path-derived module identity, source path, imports, public types and
constructors, public schemas, public module immutable values, public member
aliases, public functions, attached documentation comments, and function
contract clauses. A public module value is rendered as its name and written
type annotation. Its initializer is not rendered. ADR-lite records have a
separate section and retain their parsed anchor.

Canonical public declaration signatures preserve accepted singleton and
finite-union variant-refinement annotations. They also canonicalize the
complete signature around those annotations, including rendering `Unit` as
`()`, for function parameters, function results, and constructor fields.

A documentation line comment attaches only when immediately above the nearest
module, public type, public schema, public module immutable value, public
member alias, or public function. The `##` marker is removed. Executable
doctest and expected-output fences remain visible except hidden setup lines
beginning with `> `, which are omitted from the rendered example.

Schema references use `{@schema Name}` or a qualified path. Bare references
may resolve private or public schemas in the same module. Qualified references
select a module through the normal import rules in
[Name Resolution](name-resolution.md#explicit-import-aliases). Both an explicit
alias and the import's full written path can select a public schema or public
schema alias, including for a declared package dependency. An exact written
path selects its module before a same-spelled qualifier. If the exact module is
ambiguous or does not contain the requested schema, resolution fails without
falling through to that qualifier. Invalid, reserved, syntax-recovered, or
duplicate-target imports cannot supply a module.
Resolved references render as code; missing, private, and wrong-kind references
are name diagnostics at the referenced span, including in comments attached to
private declarations.

## Gates and write policy

The command writes no document when any selected source has parse diagnostics,
manifest validation errors, or semantic diagnostics such as source-path casing
errors. Diagnostics are written to stderr and the command exits unsuccessfully.
Errors in excluded companions or excluded sources do not block the selected
document.

A selected module immutable value declaration is checked using the shared
finite-data initializer, annotation, imported lookup, package-aware visibility,
and package-wide dependency rules. The runtime-only
`module.value_not_executable` diagnostic does not block documentation. Any
other error for a selected module value blocks the complete document. A valid
public value publishes its name and written annotation without its initializer;
a valid private value is omitted.

Variant-refinement errors remain semantic diagnostics when recovery retains a
type or constructor identity. Such an error produces no generated document.
A private named type in a public module value annotation, or a private
refinement in any structural position of a public module value annotation,
public function parameter or result, or public variant payload of a public
source ADT therefore blocks the complete document.
Publication quarantine also blocks the document
when an ordinary named type in a public signature depends on an ADT rejected
for such an exposure. The ordinary dependency does not add another privacy
diagnostic; the causal `type.variant_refinement_private` diagnostic remains the
reported failure. The [type specification](types.md#compatibility-and-limits)
defines the structural traversal and transitive publication boundary.

## Limits

The command emits package/tool metadata even when source selection is empty.
It does not include declarations, comments, ADR-lite records, or schema
reference diagnostics from excluded companions.
The command has no JSON output mode.

## References

Implementation: `crates/veln-cli/src/commands/doc.rs`. Documentation selection
and metadata behavior is covered under `examples/specification/doc/`. The
[`module-value-publication`](../../examples/specification/doc/module-value-publication/)
case checks the public signature and documentation while excluding initializer
and private-value text. The
[`module-value-publication-rejected`](../../examples/specification/doc/module-value-publication-rejected/)
case checks that a module-value type error produces no partial document. The
`dependency-schema-import-selection`, `dependency-schema-exact-precedence`,
`dependency-schema-exact-missing-no-fallback`,
`dependency-schema-ambiguous-exact-no-fallback`,
`dependency-schema-import-quarantine`, and
`dependency-schema-recovered-import` cases cover dependency schema selection
and its failure boundaries.
The refinement-specific examples under
`examples/specification/doc/adt-variant-refinement-command-*/` cover accepted
canonical signatures, rejection, recovery, nested structural visibility, and
transitive publication closure. The structural-visibility and
publication-closure command cases require empty standard output, a causal
privacy diagnostic, and no generated document content.
