# HTTP/2 Standard Library Completion and Fixture Retirement

Status: proposed

Current implemented behavior is specified by
[`http2.md`](../specification/http2.md) and its focused executable routes.
This proposal tracks only the replacement-evidence gate left open when the
broad `http2-protocol-core` fixture was removed.

## Implemented Boundary

The standard `http2::core` and `http2::hpack` modules own reusable connection,
stream, production HPACK, receive, send, flow-control, content-length,
shutdown, and output-buffer behavior. Production receive and send paths do not
use fixture state or the retired HPACK compatibility failure.

The guarded standard-package test and the focused core cases cover the current
public boundary independently of the retired fixture.

## Open Evidence Gate

The historical aggregate contains 652 `require_*` invocations, 2,044 exact
stdout lines, and 315 output tables. The former retirement checker selected
evidence from caller names, helper names, diagnostic ids, first output tokens,
or table-name prefixes. It then checked only that the selected test declaration
existed. That classified 3,011 items into 65 targets without comparing the
historical values or emitted bytes.

[`check-http2-retirement-evidence`](../../scripts/check-http2-retirement-evidence)
now reconstructs the same item inventory but accepts stdout and output-table
evidence only when the complete historical value is present in a retained
executable case. Helper invocations remain unclassified until an item-level
manifest compares their arguments and protected projections with executable
assertions. The checker must fail while any item is unclassified.

Use `--inventory` to print the stable key and value hash for every item.

## Completion Work

- Add focused or parameterized executable evidence for each distinct
  historical endpoint role, starting state, transition, diagnostic precedence,
  result projection, emitted byte sequence, and failure-atomicity invariant.
- Add an item-level helper manifest only after its format compares historical
  argument values with the retained executable assertion; helper or caller
  names alone are not evidence.
- Permit consolidation only when the checked assertion proves the same
  invariant for all mapped values.
- Keep production HPACK behind `http2::hpack`; do not restore fixture state,
  fallback decoding, canned encoding, or duplicate representation slices.
- Run the evidence checker, focused protocol cases, guarded standard-package
  tests, loader checks, and workspace gates before archiving this proposal.

## Completion Boundary

This proposal may move to `../reference/implemented-proposals/` only when the
checker reports zero unclassified items and replacement coverage passes
without the historical fixture. Until then, fixture deletion is an
implementation fact, not proof that its assertion surface was preserved.
