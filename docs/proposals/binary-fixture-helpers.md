# Binary Fixture Helpers

Status: proposed

This proposal defines fixture helpers for binary examples and tests. It is a
prerequisite for the HTTP/2 binary schema design driver because frame fixtures
need compact, reviewable source data and stable expected output.

Implemented slices: `byte_chunk_from_hex(text)` decodes compact ASCII hex
fixture text into ordinary `ByteChunk` values and reports stable
`fixture.hex.invalid_character` and `fixture.hex.odd_length` `Err(String)`
values. When those fixture text validation failures propagate out of
`run --json`, result details include the fixture text span, decoded
`ByteOffset`, nibble position, and nearby fixture text context. Executable
specification cases can also own named binary fixture records in the example
tree, with harness checks for complete lowercase hex output, decoded byte
counts, optional consumed counts, invalid fixture error text, and valid
fixture bytes that are intentionally too short for a closed-input read. The
remaining proposal work covers invalid field cases, structured field paths for
codec and protocol diagnostics, and protocol-facing fixture harness support.

## Problem

Binary protocol tests are difficult to review when byte arrays are spelled as
long lists of integers. HTTP/2 examples need valid and invalid frame fixtures,
truncated input cases, unknown frame type cases, and expected encoded output.

The repository already uses executable examples and checked fixtures as primary
specification evidence where practical. Binary schema work needs equivalent
fixture support.

## Scope

Define remaining support for:

- protocol-facing expected output chunk lists
- invalid field cases
- fixture diagnostics that can match codec and protocol field paths

## Discussion Result: Fixture Byte Rendering

Fixture command output should reuse the standard byte diagnostic rendering for
errors: bounded lowercase hex previews with byte counts and explicit
truncation. Hex validation failures report the fixture text span and the
decoded byte offset separately, so compact source data does not hide where a
bad nibble, separator, or odd byte pair was found.

Expected-output JSON for fixture assertions may include complete hex strings
for `ByteChunk` values and lists of chunks because that surface is for exact
machine comparison, not human diagnostics. Human output should still prefer
bounded previews and counts unless the command explicitly asks to print a
fixture value.

## Discussion Result: Fixture Hex Validation

Hex fixture text should accept only ASCII hex byte pairs and ASCII whitespace
between pairs. It should not accept `0x` prefixes, underscores, comments, or
format-specific separators inside the fixture text. Keeping the accepted
surface narrow makes copied protocol bytes reviewable and prevents fixture
helpers from becoming a second binary serialization language.

Validation scans the written text left to right and forms one decoded byte
from every two hex nibbles. Whitespace is allowed only between complete bytes.
An unexpected non-hex character reports `fixture.hex.invalid_character` at the
character span. A dangling nibble reports `fixture.hex.odd_length` at the
dangling nibble span. Both diagnostics include the decoded `ByteOffset` that
would contain the failed byte and, when applicable, whether the failed nibble
was the high or low nibble of that byte.

The primary human message should name the failed hex fact at the source span.
Decoded byte offset, nibble position, and any surrounding byte preview belong
in structured details or related notes. These fixture diagnostics are distinct
from codec and schema diagnostics: invalid fixture text fails before any codec
runs, while valid fixture bytes can still produce ordinary codec or protocol
failures later.

## Discussion Result: Fixture Offset Visibility

Fixture helpers should preserve two separate locations: the source span of the
fixture text and the decoded byte offset inside the produced `ByteChunk`.

The source span is used only for validating fixture text itself, such as an
invalid hex character or a dangling nibble. Once fixture text has decoded
successfully, schema, codec, and protocol diagnostics report offsets in the
decoded byte stream, not positions in the compact source literal. This keeps
fixture formatting changes from rewriting expected byte offsets.

Named fixture values may provide an optional base `ByteOffset` when a test
represents a slice from a larger stream. The default base is zero. Codec
helpers pass that base offset into decode operations and expected-output
checks compare absolute offsets after the base has been applied. Helpers must
not hide offset arithmetic behind broad "fixture failed" assertions; tests
should be able to assert the diagnostic id, absolute byte offset, field path,
consumed `ByteCount`, and any related source span separately.

Human output may include a short fixture name for orientation, but the primary
failed fact remains the codec or protocol fact at the byte offset. JSON output
keeps fixture name, fixture source span, base offset, and reported byte offset
as distinct fields when they are available.

## Discussion Result: Fixture Placement

Binary fixtures should use both Veln source and TOML case metadata, with
separate ownership.

Veln source owns named fixture values, compact hex-to-byte helper calls, and
the executable decode or encode assertions that exercise public language and
library surfaces. This keeps examples reviewable as Veln programs and makes
byte offsets, field paths, consumed counts, and returned chunks visible at the
same boundary as the code under test.

TOML case metadata owns harness selection, command arguments, expected stdout
or stderr fragments, JSON path checks, fixture file lists, and other
toolchain-facing expectations. It may carry small inline hex strings only when
the command surface under test consumes external binary input rather than a
Veln fixture value.

Shared fixture data should stay test-owned. Examples may share helper modules
or fixture files inside the example or toolchain fixture tree, but those
helpers are not exported as production APIs. Production binary APIs should be
introduced through the byte standard-library and schema proposals instead.

## Discussion Result: Shared Fixture Ownership

Examples may share binary fixture data through test-local helper modules and
fixture files, but shared fixture support should stay inside the example or
toolchain fixture tree that owns the cases. The shared module may expose named
bytes, expected decoded values, expected diagnostics, and assertion helpers
that are useful for multiple cases in that tree.

Those helpers are not part of the package's public API, are not imported by
ordinary application code, and are not documented as standard-library support.
A shared helper that becomes useful outside tests must be promoted deliberately
through the byte standard-library, schema, codec, or diagnostics proposals
instead of being exported from a fixture directory by accident.

Fixture sharing should preserve case readability. A test case should still
show the protocol fact it is exercising: frame kind, payload bytes, expected
offset, expected field path, expected consumed count, or expected diagnostic
id. Helpers may remove repeated byte construction and assertion plumbing, but
they should not collapse a fixture into an opaque "valid frame" or "bad
frame" call that hides the byte-level behavior under test.

The boundary is therefore ownership-based: fixture helpers can be reused within
the examples that own them, while production APIs come only from the standard
library and implemented language surfaces. The implemented compact hex helper
is deliberately part of the standard prelude byte vocabulary; broader shared
fixture records and assertion helpers remain test-owned until promoted through
a separate implemented surface.

## Non-Goals

- Do not define schema syntax.
- Do not define the full HTTP/2 conformance suite.
- Do not require external test data.
- Do not treat fixture helpers as general-purpose binary serialization APIs.

## Completion Criteria

- Remaining examples include named valid and invalid binary fixtures beyond
  compact source text.
- Codec and protocol fixture diagnostics with structured field paths are
  distinct from fixture text validation diagnostics.
- Test cases can assert byte offsets, field paths, consumed counts, and output
  chunks.
- The HTTP/2 design driver can add frame fixtures without unreadable byte-array
  noise.
