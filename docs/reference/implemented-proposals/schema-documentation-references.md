# Schema Documentation References

Status: implemented

This record preserves the completed documentation-comment schema reference
slice from `schema-declaration-surface.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/commands.md`, and the checked executable examples under
`../../../examples/specification/doc/`.

## Outcome

Documentation comments may write schema references as `{@schema Name}` or
`{@schema module::Name}`. The `doc` command resolves those references through
schema-aware lookup instead of ordinary value or type lookup.

Bare references resolve schemas and public schema aliases in the same module.
Qualified references require a matching written `use` path, including nested
module paths such as `use app::nested`, and resolve only public schemas or
public schema aliases. Missing targets, private imported schemas, functions,
source ADT types, and codec targets are rejected at the documentation reference
span. Documentation references do not make schema-local field names, generated
helper names, codec names, or ordinary source type bindings visible.

Resolved references render as code text in generated Markdown documentation.

## Evidence

- `../../../examples/specification/doc/schema-references/` checks
  same-module public and private schemas, imported public schemas through
  direct and nested written `use` paths, and imported public schema aliases.
- `../../../examples/specification/doc/schema-reference-diagnostics/` checks
  missing, private, wrong-kind, schema-local field, and generated helper
  reference diagnostics.

## Remaining Work

The broader schema declaration proposal remains open for schema-aware
references from later schema composition surfaces beyond codec declaration
heads, public schema member aliases, documentation comments, and binary
fixture metadata.
