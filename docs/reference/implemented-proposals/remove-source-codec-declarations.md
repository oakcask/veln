# Remove Source Codec Declarations

Status: implemented

This record preserves the completed source-level `codec` declaration removal
slice from `../../proposals/schema-binary-pattern-boundary.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/source-surface-full.md`, and executable examples under
`../../../examples/specification/`.

## Outcome

Top-level `codec Name for Schema ...` and
`pub codec Name for Schema ...` declarations are no longer accepted source
syntax. The parser reports `parse.codec_declaration_removed` at the `codec`
token and points migration toward ordinary functions plus explicit schema
`decode Schema from view at base_offset` and `encode Schema from value`
expressions.

Source-visible examples that formerly taught codec declaration calls now use
ordinary functions or explicit schema operations. Compatibility-only runtime
diagnostic ids under `codec.*` remain available where existing runtime values
use them; those ids are not source declaration syntax.

## Evidence

- `../../specification/source-surface-fixtures/rejected/codec-declaration.veln`
  rejects private, public, and qualified codec declaration heads.
- `../../../examples/specification/check/codec-declaration-diagnostics/` and
  `../../../examples/specification/check/codec-declaration-human/` check the
  JSON and human migration diagnostic.
- `../../../examples/specification/fmt/codec-declarations/` checks that
  formatter-facing command paths reject codec declarations before formatting.
- `../../../examples/specification/run/schema-decode-expression/` and
  `../../../examples/specification/run/schema-encode-expression/` check the
  current explicit schema operation routes.

## Remaining Work

The broader schema binary pattern boundary proposal remains open for
source-visible generated helper cleanup and representation-local diagnostic
reclassification.
