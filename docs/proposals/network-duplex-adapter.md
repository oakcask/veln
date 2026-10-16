---
role: proposal
update-when: The transport failure contract or planned net stream adapter changes.
---

# Typed network duplex adapter

## Outcome

`transport::DuplexStream` operations will return typed network results. A read
will return `Result<Option<ByteChunk>, net::NetError>`, where `Some` contains a
non-empty chunk and `None` means clean end. A write will return
`Result<(), net::NetError>`. This contract lets protocol code distinguish clean
end from a network failure without converting structured network information
to a string or a host runtime failure.

The standard library will adapt one caller-owned `net::Stream` to that effect.
The adapter will use the nominal `net::IO` effect. It will preserve the
`net::NetError` returned by reads and complete each supplied chunk through
`net::write_all`. It will reject a successful empty read as a typed
non-progress failure. The adapter will perform I/O only on the captured stream
and will not resolve, listen, accept, connect, shut down, or close it. The
caller retains lifecycle ownership.

HTTP/2 connection APIs will expose a transport-failure variant that preserves
the `net::NetError`. TCP service and client APIs will expose network-failure
variants and preserve their existing accepted or completed progress counts.
Their effect rows will name `net::IO`. A network failure from connection setup,
stream I/O, or lifecycle cleanup will not be reported as a protocol,
application-callback, or task-join failure. If cleanup fails while reporting a
prior outcome, the cleanup failure will be the returned network failure and
will retain the progress count from that prior outcome.

This proposal does not remove compiler-known network lookup. It does not give
the adapter ownership of stream lifecycle, and it does not install
`net::system()` below a production application boundary.

The current [effects](../specification/effects.md),
[standard-library networking](../specification/standard-library-networking.md),
and [HTTP/2](../specification/http2.md) specifications remain authoritative
until this proposal is implemented.

## Related work

### Duplex I/O and lifecycle

- **Identity and locator:** The Go Authors, [`net.Conn` in the `net` package
  source](https://go.dev/src/net/net.go#L121).
- **Relevant finding:** `net.Conn` combines read and write on one connection,
  reports operation errors, and provides close as a separate lifecycle
  operation.
- **Applicability and limits:** Go uses method-returned `error` values and puts
  lifecycle operations on the same interface. Veln uses algebraic effects and
  a structured `net::NetError`; Go therefore does not decide Veln's effect
  result type or error variants.
- **Veln consequence:** Veln adopts one duplex resource and typed operation
  failure, but keeps lifecycle outside `transport::DuplexStream`. The network
  adapter borrows the caller's stream and preserves the existing structured
  network error instead of introducing a second lossy error representation.

### Alternatives

- Keeping infallible `DuplexStream` operations would force the adapter to trap,
  invent an end-of-stream result, or discard a network failure. The proposal
  rejects those mappings because none preserves `net::NetError`.
- Adding a string-only transport error would make the effect independent of
  the network module, but it would discard operation, address, category, and
  progress fields already carried by `net::NetError`. The proposal rejects
  that loss for the only planned concrete adapter.
- Giving `DuplexStream` close or shutdown operations would let protocol code
  take lifecycle ownership. The proposal rejects that expansion because the
  service and client boundaries already own cleanup and progress reporting.

## Acceptance model

| Input or event | Required observation | Intended evidence |
| --- | --- | --- |
| A duplex read receives a non-empty chunk, clean end, or `NetError` | The effect returns `Ok(Some(chunk))`, `Ok(None)`, or the unchanged typed error, respectively | standard-library companion tests |
| A duplex read receives a successful empty chunk | The effect returns a typed non-progress error instead of allowing an unchanged protocol loop | standard-library companion test |
| A duplex write receives zero, one, or several chunks, including partial host writes | It completes every chunk in list order through `net::write_all`, performs no write for an empty list, and returns the first typed failure | standard-library companion tests |
| The adapter is given one of two distinguishable streams | Every read and write targets only the captured stream; no resolve, listen, accept, connect, shutdown, or close operation occurs | scoped fake `net::IO` handler in separate Veln test support and standard-library companion tests |
| HTTP/2 connection I/O fails | The public connection or application boundary returns a transport failure containing the same `NetError`, without classifying it as protocol or callback failure | HTTP/2 standard-library companion tests |
| HTTP/2 TCP accept, connect, stream I/O, or cleanup fails | The public service or client failure contains the `NetError` and the accepted or completed count at that boundary | HTTP/2 standard-library companion tests |
| Cleanup fails after another service or client outcome | The cleanup network failure is observable and retains the prior outcome's accepted or completed count | HTTP/2 standard-library companion tests |
| A production entry point performs HTTP/2 network I/O | The entry point installs `net::system()`; lower protocol and adapter APIs remain handler-polymorphic over `net::IO` | focused executable specification case only where this is an independently identified host-integration contract |

Add executable evidence before promoting this behavior into the effects,
networking, or HTTP/2 specifications. Keep deterministic fake handlers in
separate Veln test support. Do not replace standard-library API coverage with a
toolchain case.
