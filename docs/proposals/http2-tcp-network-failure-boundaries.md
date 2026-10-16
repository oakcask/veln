---
role: proposal
update-when: The planned HTTP/2 TCP client network-failure, cleanup-precedence, completed-count, or production-handler boundary changes.
---

# HTTP/2 TCP network failure boundaries

## Outcome

The HTTP/2 TCP client API will expose a network-failure variant that contains
`net::NetError`. A network failure from connect, stream I/O, or lifecycle
cleanup will not be reported as a protocol, application-callback, or task-join
failure. The client failure will preserve its completed-request count.

If client cleanup fails while reporting an earlier outcome, the cleanup network
failure will be returned with the completed-request count from that earlier
outcome. The public TCP client effect row will name `net::IO`. A production
entry point will install `net::system()`; the service, client, direct protocol
drivers, and stream adapter will remain handler-polymorphic.

This proposal does not give the adapter ownership of stream lifecycle. It does
not remove compiler-known network compatibility.

## Related work

### Duplex I/O and lifecycle

- **Identity and locator:** The Go Authors, [`net.Conn` in the `net` package
  source](https://go.dev/src/net/net.go#L121).
- **Relevant finding:** `net.Conn` reports operation errors and exposes close
  as a lifecycle operation separate from reads and writes.
- **Applicability and limits:** Go does not define Veln's effect rows, progress
  counts, or cleanup-failure precedence.
- **Veln consequence:** The remaining client work keeps lifecycle ownership at
  that boundary and reports cleanup as a typed network outcome instead of
  folding it into protocol or callback failures.

## Acceptance model

| Input or event | Required observation | Intended evidence |
| --- | --- | --- |
| HTTP/2 TCP client connect, stream I/O, or cleanup fails | The public client failure contains the `NetError` and the completed count at that boundary | HTTP/2 standard-library companion tests |
| Cleanup fails after another client outcome | The cleanup network failure is observable and retains the prior outcome's completed count | HTTP/2 standard-library companion tests |
| A production entry point performs HTTP/2 network I/O | The entry point installs `net::system()`; lower protocol, service, client, and adapter APIs remain handler-polymorphic over `net::IO` | focused executable specification case only where this is an independently identified host-integration contract |

Add standard-library companion evidence before promoting client failure
behavior into the HTTP/2 specification. Keep deterministic fake
handlers in separate Veln test support. Do not replace standard-library API
coverage with a toolchain case.
