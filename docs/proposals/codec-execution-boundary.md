# Codec Execution Boundary

Status: proposed

This proposal separates schema declarations from executable decoding and
encoding. It is a prerequisite for the HTTP/2 binary schema design driver
because incremental protocol parsing needs explicit readiness, consumed byte
counts, and state transitions.

## Problem

A schema can describe an external representation boundary, but HTTP/2 parsing
also needs executable behavior:

- decode from the bytes currently buffered
- report how many bytes were consumed
- distinguish incomplete input from invalid input
- preserve undecoded suffix bytes
- encode typed values into output chunks
- attach byte offsets and field paths to failures

Treating schema declarations as mutable cursors would make the source model
larger and less consistent with Veln's immutable value style.

## Scope

Define codec support for:

- decoding from `ByteView` plus an explicit input position
- encoding into immutable output chunks
- consumed byte counts
- incomplete input readiness
- invalid input errors
- decoder and encoder state values
- schema-driven codec functions
- structured diagnostics suitable for tests and agents

## Required API Decisions

The proposal must resolve:

- whether codec functions are generated, explicitly declared, or both
- whether incomplete input uses `Result`, a dedicated transition type, or a
  separate readiness value
- how absolute byte offsets are kept separate from bounded buffers
- how consumed input is dropped safely from the next parser state
- how encoding reports partial output and failures
- how codecs are named and imported

## Non-Goals

- Do not define the schema source syntax itself.
- Do not define protocol state machines.
- Do not require a socket or asynchronous runtime.
- Do not make mutable byte builders part of the source model.

## Completion Criteria

- Examples show decode, encode, consumed byte counts, and `NeedMore` behavior.
- Codec failures include structured diagnostic data.
- Incremental examples keep only undecoded suffix bytes in parser state.
- The HTTP/2 design driver can express `decode_step` as a pure state
  transition over byte input events.
