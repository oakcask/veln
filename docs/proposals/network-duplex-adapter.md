---
role: proposal
update-when: The transport failure contract or planned net stream adapter changes.
---

# Network duplex adapter

## Outcome

The standard library will adapt one caller-owned `net::Stream` to
`transport::DuplexStream`. The adapter will read through `net::IO` and will
write through `net::write_all`. It will not resolve, listen, accept, connect,
shut down, or close the captured stream. The caller retains lifecycle
ownership.

The target is blocked until `transport::DuplexStream` has a decided typed
failure result. Its current operations cannot represent `NetError`, so an
implementation cannot preserve the network failure contract without choosing
an unapproved runtime-failure mapping.

## Related work

Go's [`net.Conn`](https://go.dev/src/net/net.go#L121) keeps duplex byte I/O on
one connection abstraction and specifies that close unblocks pending I/O.
Veln adopts the narrow byte-stream boundary but keeps effect handling and
caller-owned lifecycle explicit. Go does not decide how Veln should map typed
`NetError` values into `transport::DuplexStream`.

## Acceptance model

| Input or event | Required observation | Intended evidence |
| --- | --- | --- |
| Protocol code uses a captured `net::Stream` as `transport::DuplexStream` | The adapter uses `net::IO` and `write_all`, preserves the decided failure contract, and does not close or take ownership of the stream | deterministic handler conformance test |

Add executable evidence before promoting this behavior into the transport or
network specification.
