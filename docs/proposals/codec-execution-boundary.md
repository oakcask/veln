# Codec Execution Boundary

Status: superseded

This proposal is superseded by
[Schema Binary Pattern Boundary](schema-binary-pattern-boundary.md).

Do not use this page as planned language behavior. The older design kept a
source-level `codec` declaration family for naming decode and encode
directions, binding hand-written functions, deriving schema-backed helpers,
and controlling imports. The replacement design removes that source surface:
binary `schema` declarations become explicit byte-pattern operations used from
ordinary functions.

## Replacement Direction

Use [Schema Binary Pattern Boundary](schema-binary-pattern-boundary.md) for new
design work. It keeps the useful execution concepts from this page while
removing the extra declaration family:

- `DecodeStep<T>`, `DecodeReadiness`, and structured decode failures remain
  ordinary values that protocol code can match.
- Bounded `ByteView` input plus explicit `ByteOffset` remains the diagnostic
  and consumed-count boundary for incremental decoding.
- Schema-backed decode and encode stay available through explicit schema
  operations that cite a schema directly.
- Public protocol APIs are ordinary functions, not `pub codec` items.
- Projection from schema-local visible records into domain values is ordinary
  Veln code.
- Representation-local failures should be schema-owned diagnostics, not
  wrapper-level codec diagnostics.

## Historical Scope

The superseded design covered:

- top-level `codec` and `pub codec` declarations
- `decode` and `encode` direction lists
- `derive decode`, `derive encode`, `decode with`, and `encode with` clauses
- imported public codec call behavior
- generated-helper-backed codec wrappers around binary schema helpers
- hand-written codec consumed-count validation and partial encode resume
- codec-owned diagnostics for wrapper failures

Those ideas should not be extended in new proposal work. If a future design
needs source-visible parser or encoder state, model it with ordinary functions
and explicit state values around schema pattern operations.

## Preserved Decisions

The replacement direction keeps these decisions, but moves them out of a
`codec` declaration surface:

- incomplete input is a normal `NeedMore` transition, not malformed input
- successful decode reports a consumed `ByteCount`
- `Invalid` consumes no input at the schema operation boundary
- absolute byte offsets are supplied explicitly by the caller
- retained byte ranges must be visible as `ByteView` or `ByteChunk` values
- stateful or budgeted encoding belongs in ordinary values returned from
  ordinary functions

Implemented codec slices remain archived under
`../reference/implemented-proposals/` as historical implementation records.
They are not the preferred route for new schema, HTTP/2, or binary-pattern
design work.

## Migration Target

Current and future proposal work should migrate from:

```veln
pub codec HeaderCodec for HeaderWire decode encode
	derive decode
	derive encode
end
```

to ordinary public functions that cite schema operations directly:

```veln
pub fn decode_header(input: PendingInput) -> DecodeStep<HeaderWireRecord>
	decode HeaderWire from input.bytes at input.base_offset
end

pub fn encode_header(header: HeaderWireRecord) -> Result<ByteChunk, EncodeError>
	encode HeaderWire from header
end
```

The exact expression spelling is still owned by
[Schema Binary Pattern Boundary](schema-binary-pattern-boundary.md). This page
only preserves the old link target and explains why new work should not add
more `codec` syntax.
