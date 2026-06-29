# Binary Schema Imported Converter Bare Inverse Encode

This record covers the completed generated encode slice for binary schema
mapping assignments whose forward converter is an imported public pure
function written without an explicit qualified path at the call site.

Implemented behavior:

- Schema mapping converter calls resolve a same-module converter first, then a
  single unambiguous public function imported into the schema module by a
  `use` declaration.
- Public function aliases are valid converter names. The generated mapping
  retains the aliased target function identity, so encode projection calls the
  underlying converter and inverse implementation.
- A generated `byte_encode_<schema>` helper can project a mapped target field
  through an imported public converter written as an unqualified imported name
  when the assignment also names an explicit imported public inverse converter
  by the same name-resolution rules.
- The inverse projection still checks that the recovered schema-local value
  round-trips through the forward converter. Failed round-trips keep
  `codec.encode_mapping_mismatch` at the mapped target field path.
- Derived encode codec boundaries keep the same generated-helper eligibility
  when converter and inverse names are imported through public function
  aliases.

Executable specification evidence:

- `examples/specification/run/binary-schema-imported-mapped-converter-bare-encode/`
- `examples/specification/run/binary-schema-imported-mapped-converter-bare-encode-mismatch/`
- `examples/specification/run/derived-codec-imported-mapped-converter-alias-encode-boundary/`

This slice does not infer inverse converter names, add converter arities, add
mapping expression forms, change decode mapping semantics, or accept ambiguous
bare imported converter names.
