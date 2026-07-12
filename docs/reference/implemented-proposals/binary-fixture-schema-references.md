# Binary Fixture Schema References

Status: implemented

This record preserves the completed binary fixture schema-reference slice
from the schema declaration proposal. Current behavior is specified by
`../../specification/execution.md` and checked by the executable cases under
`../../../examples/specification/run/binary-fixture-schema-references/` and
`../../../examples/specification/run/binary-fixture-schema-reference-diagnostics/`.

## Completed Behavior

Executable specification fixtures may associate diagnostic expectations with
a schema or schema alias resolved from the command source graph. Local bare
references and imported public qualified references use schema-aware lookup.
Invalid, inaccessible, non-schema, and generated-helper targets are rejected,
and a supplied field path must start with the resolved schema name.

## Evidence

- The positive case covers a private local schema, an imported public schema,
  and an imported public schema alias.
- The diagnostic case covers missing and private schemas, function and source
  ADT targets, codec and generated-helper names, missing imports, and a field
  path whose first segment names the wrong schema.
