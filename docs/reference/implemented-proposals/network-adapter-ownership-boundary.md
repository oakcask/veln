# Network Adapter Ownership Boundary

Status: implemented

This record preserves the completed adapter-owned listener-to-clean-stream-end
lifecycle slice from [external production socket runtime record](network-effect-integration-boundary.md).
Current behavior is specified by `../../specification/names-effects.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable case under
`../../../examples/specification/run/socket-stream-adapter-owned-lifecycle/`
and its matching effect check under
`../../../examples/specification/check/socket-stream-adapter-owned-lifecycle-effects/`.

## Outcome

The completed slice keeps the transport boundary fixture-backed while making
adapter ownership explicit. The checked executable case has one adapter-shaped
entry path that creates and owns a `NetListener`, accepts with
`net::accept_or_end`, owns the accepted `NetStream`, reads chunks until clean
stream end with `net::read_chunk_or_end`, routes ordinary `StreamInput` values
through the standard channel API, invokes a plain handler, and projects
`SendBytes` response actions back to ordered `net::write_chunk` calls.

The adapter declares only the existing coarse `net` and `concurrency` effects.
The handler remains ordinary source code: it receives stream values and
explicit state, does not receive socket handles, and does not call `net`
functions. The executable cases check both the source-visible result and the
ordered fixture event log, including listen, accept, reads, clean read end, and
writes, and reject adapter declarations that omit either `net` or
`concurrency`.

## Read When

- Auditing why the adapter-owned lifecycle slice is no longer active proposal
  work.
- Checking completion evidence before changing the network integration
  proposal route.

## Skip Unless Needed

- Do not read this page for ordinary current network, execution, or effect
  behavior.
- Use the specification pages and checked executable case for current behavior.
