---
role: specification
authority: normative
update-when: The veln doc command selection, generated documentation output, manifest metadata, or source-error gate changes.
---

# Doc Command

`doc` generates deterministic Markdown documentation for selected source
files. It uses the same source discovery rule as `check`: absent paths discover
`.veln` files recursively below the current project root, explicit directories
are searched recursively, and selected paths are sorted and deduplicated.
Exact `.test.veln` test companions are excluded from the generated public
document after source discovery. Explicit companion path inputs are excluded
the same way. If no non-companion source remains, `doc` still emits package and
tool metadata and the generated module section states that no source modules
were selected. `_test.veln` integration-test modules remain ordinary selected
sources for generated documentation.

`doc` reads `veln.toml` when present. The implemented manifest documentation
surface accepts string-valued `[package]` fields and string-valued
`[tool.<name>]` fields. Package fields are emitted as package metadata, and
tool fields are emitted under a tool metadata section. The package `name`
field, when present, is the generated document title; otherwise the title is
`Veln Project`.

If discovery selects no non-companion source files, `doc` still emits package
and tool metadata from `veln.toml` when present. The generated module section
states that no source modules were selected.

The command has parse, manifest, and semantic gates. If any selected source has
parse diagnostics, if manifest validation reports errors, or if a selected
non-companion source has semantic diagnostics such as source identifier casing
errors, `doc` emits human diagnostics on stderr, writes no documentation, and
exits with failure. Invalid source identifier casing in an excluded source or
excluded `.test.veln` companion is not reported by `doc` and does not block the
selected documentation set.

For `check`, `run`, `test`, and `doc`, parse-clean package-relative sources
derive local module identity from the selected `.veln` path. Path separators
become `::`. Path segments with invalid module-class initials produce
source-path identifier casing diagnostics before semantic diagnostics are
reported. Path segments that start with an ASCII lowercase letter but are not
valid module identifiers produce structural module diagnostics instead.

For each parse-clean selected non-companion source, `doc` emits the
path-derived source module identity, the source path, imports, public source
`type` declarations, public constructors, public `schema` declarations, public
member aliases, and public `fn` declarations. Public `fn` documentation
includes attached documentation line comments and contract clauses. Public
`type` and `schema` documentation includes attached documentation line
comments. Excluded companion sources contribute no module heading, source
path, imports, declarations, documentation comments, ADR-lite records, or
documentation schema-reference diagnostics.

Documentation line comments are attached to the nearest following module,
public type, public schema, public member alias, or public function
declaration only when they are immediately above that declaration. The
generated Markdown strips the `##` marker.
Executable doctest and expected-output fences remain visible examples, except
hidden setup lines whose visible doc-comment content starts with `> ` are
omitted from the generated example. ADR-lite records are emitted in a separate
ADR-lite section and keep their parsed anchor when one exists.
Documentation comments may write schema references as `{@schema Name}` or
`{@schema module::Name}`. The `doc` command resolves those references through
schema-aware lookup: same-module bare references may name private or public
schemas, and module-qualified references require a matching written `use` path,
including nested module paths such as `use app::nested`, and a public schema or
public schema alias. The generated Markdown renders a resolved schema reference
as code text. Missing, private, and wrong-kind schema references are name
diagnostics reported at the referenced name span. Schema-reference diagnostics
are validated for all documentation comments in selected non-companion sources,
including comments attached to private declarations that are not emitted in the
generated Markdown.

