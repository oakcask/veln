# HTTP/2 Production Outbound HPACK Octet Values

## Status

Implemented and archived as current behavior in
`../../specification/execution.md`.

## Implemented Behavior

The production ordered HPACK encoder keeps validated header names as strings
and header values as immutable octets. Static-name, dynamic-name, and new-name
literals preserve those bytes without text conversion. Exact static indexed
selection occurs only when the octets equal a fixed static-table value, while
exact dynamic entries are reused by byte equality.

Raw and Huffman candidates are compared by their complete encoded byte counts.
Huffman is selected only when smaller, so ties deterministically remain raw.
Literal insertion, table-size accounting, eviction, and returned state use the
exact octet count and value. A later-field failure exposes neither a partial
block nor a partially updated state.

Outbound request and response HEADERS and server-side `PUSH_PROMISE` use this
same encoder before the existing CONTINUATION splitting and peer table-size
limits.

## Executable Evidence

historical aggregate evidence covers non-visible
raw octets, Huffman-selected octets, static and dynamic representation
selection, insertion and indexed reuse, HEADERS and `PUSH_PROMISE` framing,
and input-state reuse after a late failure.

## Boundary

Header names remain validated strings. This slice does not add representation
families, unbounded dynamic tables, or fixed-width header-list variants.
