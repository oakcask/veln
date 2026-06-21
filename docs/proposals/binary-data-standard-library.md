# Binary Data Standard Library

Status: proposed

This proposal tracks the remaining binary standard-library work needed by
binary schemas, codecs, and sans-I/O protocol cores. The source-visible byte
vocabulary and stream-input slice is current behavior under
`../specification/types.md`, `../specification/names-effects.md`, and
`../specification/execution.md`; this proposal keeps the unimplemented
follow-up work.

## Problem

Current Veln programs can use `Int`, `String`, records, ADTs, `Vec`, `List`,
`Dict`, `Option`, and `Result`, but there is no source-visible byte type or
byte-buffer vocabulary. HTTP/2 requires fixed-width binary reads, length-based
slicing, offsets for diagnostics, and output chunks for encoding.

## Scope

Define the remaining standard-library support for binary-buffer behavior,
schema-facing conversion policy, protocol-facing diagnostics, and bounded
buffers for flow-control and incremental parsing examples.

The implemented narrow slice already covers `Byte`, immutable `ByteChunk`,
immutable `ByteView`, `ByteOffset`, `ByteCount`, `StreamInput`, pure helpers
for construction, length, append, bounded take, bounded drop, bounded views,
bounded view count, bounded view take/drop/slice, outgoing `List<ByteChunk>`
construction and append, view-to-chunk materialization, fixed-width unsigned
big-endian reads and writes for 8-bit, 16-bit, 24-bit, 31-bit, 32-bit, 40-bit,
48-bit, and 64-bit source-visible `Int` values, fixed-width unsigned
little-endian reads and writes for 16-bit, 24-bit, 31-bit, 32-bit, 40-bit,
48-bit, and 64-bit source-visible `Int` values, source-visible pending input
and outgoing immutable chunk
collection for protocol examples, `ByteView` freeze preservation across task
and channel boundaries, source-visible `ByteView` range failure diagnostics
with bounded byte previews, checked byte write conversion diagnostics with
helper name, supplied value, accepted range, width, and byte order,
schema-facing length-bounded `ByteView` encode conversion diagnostics with
schema field path and byte-view count mismatch reason, and
structured byte previews for the implemented schema-owned byte diagnostics and
HTTP/2 client connection preface protocol-owned byte diagnostics, plus HTTP/2
invalid frame-kind and PRIORITY self-dependency protocol-owned byte
diagnostics, plus HTTP/2 invalid stream-id domain protocol-owned byte
diagnostics, and the HPACK fixture unsupported-header-block protocol-facing
diagnostic, plus HTTP/2 SETTINGS value range protocol-owned byte diagnostics,
HTTP/2 `WINDOW_UPDATE` invalid-increment protocol-owned byte diagnostics, and
HTTP/2 unexpected SETTINGS ACK protocol-owned byte diagnostics, plus HTTP/2
header-list and header-table receive-limit protocol-owned byte diagnostics.
Current behavior belongs to the specification pages, not this proposal.

## Discussion Result: Core Byte Vocabulary Names

The byte vocabulary includes `ByteView` alongside the implemented `Byte`,
`ByteChunk`, `ByteOffset`, `ByteCount`, and `StreamInput` names.

`ByteChunk` is the immutable owned byte sequence for both input and output.
Encoding APIs should return `ByteChunk` or a list of `ByteChunk` values rather
than introduce a separate `OutputChunk` type. `ByteView` is the bounded
immutable slice vocabulary used by parsers and codecs while input remains
buffered.

`ByteOffset` is the public name for absolute byte offsets in parser state and
diagnostics. `ByteCount` is the public name for lengths, consumed counts, and
bounded buffer sizes. The library should avoid a public `BytePosition` alias
until a later design needs a position value that is not simply an absolute byte
offset.

## Discussion Result: Unsigned Width Boundary

Exact unsigned widths should be schema primitives first, not a family of
ordinary source-visible numeric types.

The standard library exposes byte-oriented checked reads and writes for the
current fixed-width unsigned big-endian representations, and their ordinary
Veln value result is `Int` unless an explicit mapping converts the value into an
independently declared domain type. This keeps external layout facts such as
width, byte order, and reserved bits at the schema or codec boundary instead of
leaking every wire width into the general type system.

Binary schema primitives own names such as `UInt1` through `UInt8`,
`UInt16be`, `UInt16le`, `UInt24be`, `UInt24le`, `UInt31be`, `UInt31le`,
`UInt32be`, `UInt32le`, `UInt40be`, `UInt40le`, `UInt48be`, `UInt48le`,
`UInt64be`, and `UInt64le`.
These names describe the external
representation that is decoded or encoded, not ordinary Veln numeric types
that can appear anywhere a value type is expected.
Little-endian variants use the same schema-primitive family when a binary
format needs them.

HTTP/2's 24-bit length and 31-bit stream identifier therefore do not require
new public `UInt24` or `UInt31` source types. The schema primitive validates
and reads the external field, then mapping code can store the result in `Int`
or in a later domain-specific type such as a stream identifier wrapper.

## Discussion Result: Byte Operation Execution Boundary

Byte operations should be ordinary pure standard-library functions at the
source surface. Programs should call names such as byte length, view, drop,
append, checked read, and checked write operations without special syntax or a
separate intrinsic-call form.

The compiler may still register these functions as compiler-known descriptors
when it needs stable type facts, bounds checks, lowering hooks, runtime entry
points, or diagnostic ids. That implementation knowledge must not change the
source-level effect: byte reads, slicing, appending immutable chunks, and
checked integer conversion are pure operations that either return values or
structured failures through the declared result type.

This boundary keeps examples and fixture helpers portable across interpreted,
compiled, and source-backed standard-library implementations. It also keeps
future optimization freedom: a backend may lower `ByteView` slicing to a
zero-copy view, a compact copied chunk, or another immutable representation as
long as the public function contract and diagnostics stay the same.

## Discussion Result: Byte Diagnostic Rendering

Human diagnostics should render byte chunks and byte views as bounded
lowercase hex byte pairs. The display form should group bytes with spaces,
show the total `ByteCount`, and mark truncation when the diagnostic only shows
a prefix. It should not render bytes as a `String`, infer text encoding, or
include an unbounded dump in the primary message.

JSON diagnostics should use structured byte preview fields instead of a
localized display string. The stable shape should include the total byte
count, the preview encoding, the preview hex data, the preview byte count, and
whether the preview was truncated. Byte offsets, field paths, expected counts,
and actual available counts stay in their own diagnostic fields rather than
being encoded into the preview text.

`ByteView` diagnostics use the same rendering rule for the bounded bytes the
view exposes. When the diagnostic has an absolute `ByteOffset`, that offset is
reported separately from the preview so dropping consumed input never hides the
location of the failed byte. Exact byte-for-byte fixture assertions may expose
full hex data in fixture JSON output, but error diagnostics remain preview
bounded by default.

## Non-Goals

- Do not define schema declaration syntax here.
- Do not implement socket reads or writes.
- Do not define HPACK table behavior.
- Do not promise production memory layout or zero-copy guarantees.

## Remaining Completion Criteria

- Specification pages describe the later binary-buffer behavior and any
  schema-facing conversion boundaries not covered by the current byte helpers,
  source-visible `ByteView` range diagnostics, checked byte write conversion
  diagnostics, schema-owned byte diagnostic previews, and command-facing
  length-bounded `ByteView` encode conversion diagnostics.
- Later protocol-facing diagnostics beyond the implemented schema-owned byte
  slices, HTTP/2 client connection preface slice, HTTP/2 invalid frame-kind
  slice, HTTP/2 invalid stream-id domain slice, HTTP/2 PRIORITY
  self-dependency slice, HPACK fixture unsupported-header-block and
  SETTINGS value range slices, the HTTP/2 `WINDOW_UPDATE` invalid-increment
  slice, the HTTP/2 unexpected SETTINGS ACK slice, and the HTTP/2 header-list
  and header-table receive-limit slice cover
  protocol-owned byte previews,
  field paths,
  expected and actual counts, and absolute offsets where those diagnostics
  inspect bytes directly.
