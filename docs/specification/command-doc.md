---
role: specification
authority: normative
update-when: The veln doc command selection, generated documentation output, manifest metadata, or source-error gate changes.
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
constructors, public schemas, public member aliases, public functions,
attached documentation comments, and function contract clauses. ADR-lite
records have a separate section and retain their parsed anchor.

A documentation line comment attaches only when immediately above the nearest
module, public type, public schema, public member alias, or public function.
The `##` marker is removed. Executable doctest and expected-output fences
remain visible except hidden setup lines beginning with `> `, which are
omitted from the rendered example.

Schema references use `{@schema Name}` or a qualified path. Bare references
may resolve private or public schemas in the same module. Qualified references
require a matching written `use` path and a public schema or public alias.
Resolved references render as code; missing, private, and wrong-kind references
are name diagnostics at the referenced span, including in comments attached to
private declarations.

## Gates and write policy

The command writes no document when any selected source has parse diagnostics,
manifest validation errors, or semantic diagnostics such as source-path casing
errors. Diagnostics are written to stderr and the command exits unsuccessfully.
Errors in excluded companions or excluded sources do not block the selected
document.

## Limits

The command emits package/tool metadata even when source selection is empty.
It does not include declarations, comments, ADR-lite records, or schema
reference diagnostics from excluded companions.

## References

Implementation: `crates/veln-cli/src/commands/doc.rs`. Documentation selection
and metadata behavior is covered under `examples/specification/doc/`.
