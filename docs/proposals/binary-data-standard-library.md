# Binary Data Standard Library

Status: proposed

This proposal defines the source-visible byte vocabulary needed by binary
schemas, codecs, and sans-I/O protocol cores. It is a prerequisite for the
HTTP/2 binary schema design driver because frame parsing requires immutable
byte chunks, bounded views, byte positions, and checked integer conversion.

## Problem

Current Veln programs can use `Int`, `String`, records, ADTs, `Vec`, `List`,
`Dict`, `Option`, and `Result`, but there is no source-visible byte type or
byte-buffer vocabulary. HTTP/2 requires fixed-width binary reads, length-based
slicing, offsets for diagnostics, and output chunks for encoding.

## Scope

Define standard-library support for:

- `Byte`
- immutable `ByteChunk`
- immutable `ByteView`
- `ByteOffset`
- `ByteCount`
- byte length, slice, drop, and append operations
- checked reads for exact-width unsigned integers
- endian-aware reads and writes
- checked integer conversion with overflow diagnostics
- immutable output chunks for encoding
- bounded buffers for flow-control and incremental parsing examples

## Required API Decisions

The proposal must resolve:

- canonical names for byte chunks, byte views, offsets, and counts
- whether unsigned integer widths are source-visible types, schema primitives,
  or both
- how to represent 24-bit and 31-bit HTTP/2 fields
- how views interact with value freezing across tasks and channels
- whether byte operations are pure functions or compiler-known intrinsics
- how byte chunks are rendered in human and JSON diagnostics

## Non-Goals

- Do not define schema declaration syntax here.
- Do not implement socket reads or writes.
- Do not define HPACK table behavior.
- Do not promise production memory layout or zero-copy guarantees.

## Completion Criteria

- Specification pages describe byte values, chunks, views, offsets, and counts.
- Examples decode and encode small binary values without relying on HTTP/2.
- Checked conversion and truncation diagnostics are covered.
- Runtime support preserves immutability across ordinary values, tasks, and
  channels.
- The HTTP/2 design driver can represent pending input and outgoing chunks in
  source examples.
