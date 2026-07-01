# Codec Execution Boundary

Status: superseded

This route is superseded. The completed replacement boundary now lives at
[../reference/implemented-proposals/schema-binary-pattern-boundary.md](../reference/implemented-proposals/schema-binary-pattern-boundary.md).

Use current behavior routes instead of this historical proposal path:

- Source syntax and schema operations:
  [../specification/source-surface.md](../specification/source-surface.md).
- Runtime schema operation behavior:
  [../specification/execution.md](../specification/execution.md).
- Implemented proposal history:
  [../reference/implemented-proposals/README.md](../reference/implemented-proposals/README.md).

## Historical Boundary

The older design kept a source-level `codec` declaration family for naming
decode and encode directions, binding hand-written functions, deriving
schema-backed helpers, and controlling imports. The replacement design removes
that source surface: binary `schema` declarations are explicit byte-pattern
operations used from ordinary functions.

Completed codec-era implementation records remain archived under
`../reference/implemented-proposals/`:

- [Codec Generated Helper Boundary Slices](../reference/implemented-proposals/codec-generated-helper-boundary-slices.md)
- [Codec Hand-Written Encode Resume](../reference/implemented-proposals/codec-hand-written-encode-resume.md)
- [Codec Hand-Written NeedEnd Boundary](../reference/implemented-proposals/codec-hand-written-need-end-boundary.md)
- [Codec Imported Hand-Written Boundary](../reference/implemented-proposals/codec-imported-hand-written-boundary.md)
- [Codec Imported Derived Boundary](../reference/implemented-proposals/codec-imported-derived-boundary.md)
- [Codec Owned Decode Invalid Id Diagnostics](../reference/implemented-proposals/codec-owned-decode-invalid-id-diagnostics.md)
- [Codec Sequence Mismatch Diagnostics](../reference/implemented-proposals/codec-sequence-mismatch-diagnostics.md)
- [Codec Version Mismatch Diagnostics](../reference/implemented-proposals/codec-version-mismatch-diagnostics.md)
- [Codec Tag Mismatch Diagnostics](../reference/implemented-proposals/codec-tag-mismatch-diagnostics.md)

The replacement boundary keeps the useful execution concepts without the
declaration family:

- incomplete input is a normal `NeedMore` transition, not malformed input
- successful decode reports a consumed `ByteCount`
- `Invalid` consumes no input at the schema operation boundary
- absolute byte offsets are supplied explicitly by the caller
- retained byte ranges must be visible as `ByteView` or `ByteChunk` values
- stateful or budgeted encoding belongs in ordinary values returned from
  ordinary functions

New schema, HTTP/2, or binary-pattern design work should cite the
specification pages and implemented records above, not extend this
source-level `codec` route.
