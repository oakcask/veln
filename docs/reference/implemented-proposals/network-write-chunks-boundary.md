# Network Write Chunks Boundary

Status: implemented

This record preserves the completed source-visible ordered chunk-list write
slice from [external production socket runtime record](network-effect-integration-boundary.md).
Current
behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked examples under
`../../../examples/specification/run/transport-socket-write-chunks-boundary/`
and
`../../../examples/specification/check/transport-socket-write-chunks-effects/`.

## Outcome

The completed slice adds `net::write_chunks(stream, chunks)` as a
source-visible standard-library operation over an adapter-owned `NetStream`
and source-owned `List<ByteChunk>`. The call uses the existing coarse `net`
effect label, returns `()`, and writes each chunk in source list order through
the same stream write path as `net::write_chunk`.

The executable run case accepts a fixture stream, builds two ordinary
`ByteChunk` values, places them in a `List<ByteChunk>`, calls
`net::write_chunks`, and checks that the fixture records two writes in list
order. The effect check keeps ownership explicit: source that writes the
chunk list to a `NetStream` must declare `net`.

Transport write failures remain runtime transport failures on the existing
stream write path. The boundary does not add an effect label, change
`net::write_chunk`, introduce new socket APIs, or move pure protocol handlers
into adapter-owned transport code.

## Read When

- Auditing why ordered source-owned chunk-list writes are no longer active
  proposal work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked examples for current behavior.
