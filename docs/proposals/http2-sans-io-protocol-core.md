# HTTP/2 Sans-I/O Protocol Core

Status: proposed

This proposal defines the finite remaining HTTP/2 protocol-core work after the
completed state-transition and production HPACK slices. Current behavior
belongs under `../specification/`; completed proposal history belongs under
`../reference/implemented-proposals/`.

## Problem

The production HPACK paths accept recursive header lists, arbitrary octet
values, legal leading table-size update sequences, the standard indexed and
literal representation families, immutable dynamic-table state, and outbound
representation selection. Some failures still collapse to the fixture-owned
generic `hpack.fixture.unsupported_header_block` result even though the
production boundary can identify the failed representation stage.

The remaining work is to replace those generic production fallbacks with
focused typed compression failures. No HTTP/2 frame or stream-state transition
is currently scheduled by this proposal.

## Finite Remaining Scope

The remaining production HPACK targets are limited to these three failure
families:

1. Indexed-field decoding that reaches the generic unsupported result instead
   of a focused malformed prefixed-integer, zero index, or unavailable table
   entry failure.
2. Literal-field decoding whose indexed or raw name, string length, raw octets,
   or Huffman payload reaches the generic unsupported result instead of the
   corresponding focused malformed or unavailable-name failure.
3. Ordered-list encoding that reaches the generic unsupported result after
   name validation, integer encoding, string encoding, or active-capacity
   selection instead of returning a focused encode failure for that stage.

Each target applies only to the production HEADERS, `PUSH_PROMISE`, and final
CONTINUATION paths. Standalone compatibility fixtures may retain fixture-owned
failures when they are deliberately testing the old boundary.

## Design Constraints

### Limit Placement

Schema validation owns representation-local facts available from the current
decoded fields. Runtime settings own negotiated or configured peer limits.
Contracts own implementation invariants and must not replace peer protocol
errors for invalid incoming frames.

### Compression Error Reporting

Pure transitions return typed compression failures. Diagnostic conversion
supplies stable ids, the absolute HPACK byte offset, representation family,
failed stage, inspected bytes, and carried table-state provenance. A failed
decode or encode exposes no partial header list, output bytes, or next state.

### HPACK Boundary

Frame decoding keeps header blocks opaque until an explicit HPACK library
boundary consumes them with immutable state. Schema declarations must not
grow an HPACK-specific special case.

## Selection Stop Rule

Do not add another target to this page by writing "remaining SETTINGS",
"remaining DATA", "remaining lifecycle", or another category. A newly found
HTTP/2 gap must name the RFC rule, endpoint role, starting state, input or send
intent, accepted or rejected transition, preserved state, and diagnostic
precedence. Add it as a finite target only after confirming that the executable
cases and current specification do not already cover it.

Extending a fixture count, stream-list width, table-update count, or other
same-shaped sequence is not a target; use the existing recursive or
list-backed abstraction instead.

## Non-Goals

- TLS, ALPN, socket listeners, or platform networking
- production throughput optimization
- unspecified SETTINGS, DATA, stream-lifecycle, or graceful-shutdown work
- new HPACK representation policy beyond the three generic-failure families
- encoding all protocol state rules inside schema declarations
- duplicating implemented behavior or completion history in this proposal

## Completion Criteria

- All three production fallback families return focused typed failures and no
  longer project `hpack.fixture.unsupported_header_block`.
- Executable cases cover the direct production decoder or encoder plus complete
  HEADERS, `PUSH_PROMISE`, and final CONTINUATION routing where applicable.
- Failed transitions preserve the input HPACK and HTTP/2 state and expose no
  partial decoded fields or encoded bytes.
- Current behavior is promoted to the smallest matching specification page.
- Completed history is archived under the implemented-proposal reference
  index and removed from this active proposal.
