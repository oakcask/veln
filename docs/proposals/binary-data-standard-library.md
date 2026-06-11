# Binary Data Standard Library

Status: proposed

This proposal tracks the remaining binary standard-library work needed by
binary schemas, codecs, and sans-I/O protocol cores. The first source-visible
byte vocabulary slice is current behavior under
`../specification/types.md`, `../specification/names-effects.md`, and
`../specification/execution.md`; this proposal keeps the unimplemented
follow-up work.

## Problem

Current Veln programs can use `Int`, `String`, records, ADTs, `Vec`, `List`,
`Dict`, `Option`, and `Result`, but there is no source-visible byte type or
byte-buffer vocabulary. HTTP/2 requires fixed-width binary reads, length-based
slicing, offsets for diagnostics, and output chunks for encoding.

## Scope

Define the remaining standard-library support for:

- immutable `ByteView`
- checked reads for exact-width unsigned integers
- endian-aware reads and writes
- checked integer conversion with overflow diagnostics
- immutable output chunks for encoding
- bounded buffers for flow-control and incremental parsing examples

The implemented narrow slice already covers `Byte`, immutable `ByteChunk`,
`ByteOffset`, `ByteCount`, and pure helpers for construction, length, append,
bounded take, and bounded drop. Current behavior belongs to the specification
pages, not this proposal.

## Discussion Result: Core Byte Vocabulary Names

The standard byte vocabulary should use `Byte`, `ByteChunk`, `ByteView`,
`ByteOffset`, `ByteCount`, and `StreamInput`.

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

`StreamInput` is the incremental input event type used by sans-I/O parsers.
It should distinguish byte chunk arrival from end-of-stream with explicit ADT
variants. End-of-stream must not be represented as a missing chunk or a
zero-length `ByteChunk`.

## Discussion Result: Stream Input Variant Names

`StreamInput` should use `Chunk(bytes: ByteChunk)` and `End` as its first
public variants.

`Chunk` names arrival of bytes from an external stream without tying the value
to sockets, files, or a particular transport. The payload type remains the
shared immutable `ByteChunk`; a separate `InputChunk` name would duplicate the
direction-neutral byte vocabulary without adding a new invariant.

`End` names the explicit end-of-stream event. Avoid `Eof` as the public
variant because the same event is useful for non-file streams, and avoid
`Closed` because a closed transport can still have protocol-specific cleanup
or error handling outside the byte-input event. A zero-length chunk is still a
chunk event and must not stand in for `End`; callers may ignore or normalize
empty chunk arrivals at their own API boundary.

## Discussion Result: Byte View Freezing

`ByteView` should use the ordinary Veln value-freezing boundary for tasks and
channels. A frozen view carries an immutable bounded byte sequence with the
same logical offset and length the sender observed; it does not carry a
source-visible borrow lifetime.

The runtime is responsible for preserving the referenced bytes across the
boundary. It may share immutable backing storage, pin storage, reference-count
storage, or copy the bounded range into a compact `ByteChunk`. The standard
library should not promise a particular memory layout or zero-copy behavior.

This means byte views remain convenient for protocol slices and fixture
helpers while preserving the existing concurrency rule that sent values and
task return values are frozen before crossing the boundary. APIs that retain or
send byte views should expose size limits so programs do not accidentally keep
large consumed input buffers alive.

## Discussion Result: Unsigned Width Boundary

Exact unsigned widths should be schema primitives first, not a family of
ordinary source-visible numeric types.

The standard library should expose byte-oriented checked reads and writes for
fixed-width unsigned representations, but their ordinary Veln value result
should be `Int` unless an explicit mapping converts the value into an
independently declared domain type. This keeps external layout facts such as
width, byte order, and reserved bits at the schema or codec boundary instead of
leaking every wire width into the general type system.

Binary schema primitives own names such as `UInt8`, `UInt16be`, `UInt24be`,
`UInt31be`, and `UInt32be`. These names describe the external representation
that is decoded or encoded, not ordinary Veln numeric types that can appear
anywhere a value type is expected. Little-endian variants should use the same
schema-primitive family when a binary format needs them.

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

- Specification pages describe byte views, checked reads and writes,
  conversion boundaries, stream input, and binary-buffer behavior.
- Examples decode and encode small binary values without relying on HTTP/2.
- Checked conversion and truncation diagnostics are covered.
- Runtime support preserves byte views across tasks and channels.
- The HTTP/2 design driver can represent pending input and outgoing chunks in
  source examples.
