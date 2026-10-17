---
role: proposal
update-when: The planned HTTP/2 production TCP client handler boundary changes.
---

# HTTP/2 production TCP network entry point

## Outcome

A production HTTP/2 TCP client entry point will install `net::system()` around
the handler-polymorphic client service. The lower client, direct protocol
drivers, and stream adapter will continue to accept caller-supplied handlers.
This proposal does not remove compiler-known network compatibility.

## Related work

### Standard network system handler

- **Identity and locator:** Veln's current
  [standard-library networking specification](../specification/standard-library-networking.md#usage).
- **Relevant finding:** `net::system()` handles `net::IO` and exposes the host
  network and clock effects to the application.
- **Applicability and limits:** The handler contract defines host selection but
  does not choose the HTTP/2 entry-point name or its request configuration.
- **Veln consequence:** The production entry point will install this handler.
  Lower HTTP/2 APIs will remain polymorphic over `net::IO`.

## Acceptance model

| Input or event | Required observation | Intended evidence |
| --- | --- | --- |
| A production entry point performs HTTP/2 network I/O | The entry point installs `net::system()`; lower protocol, service, client, and adapter APIs remain handler-polymorphic over `net::IO` | focused executable specification case only where this is an independently identified host-integration contract |

Keep standard-library API behavior in companion tests. Use a focused
executable specification case only for the independently identified
host-integration boundary.
