# Proposals

Status: routing

This directory contains only planned or incomplete work. Proposal text is not
current language behavior unless the matching page under
`../specification/` also states it.

## Read First

- Current behavior: [Language Specification](../specification/README.md).
- Completed proposal history:
  [Implemented Proposal Records](../reference/implemented-proposals/README.md).

## Catalog

- [HTTP/2 Sans-I/O Protocol Core](http2-sans-io-protocol-core.md): uncovered,
  bounded HTTP/2 state transitions, SETTINGS and DATA interactions,
  stream-lifecycle rules, graceful shutdown, and HPACK gaps.
- [Network Effect Integration Boundary](network-effect-integration-boundary.md):
  production socket ownership, richer stream-task lifecycle, and transport
  adapter APIs beyond the checked loopback boundary.
- [Schema Declaration Surface](schema-declaration-surface.md): binary field
  shapes outside current generated-helper eligibility and a later explicit
  schema-composition surface.

## Selection Rule

Before implementing a proposal slice, compare it with the matching
specification page and executable cases. Do not select work that is already
covered there or only extends a numbered, width-based, arity-based, route-count,
or diagnostic-id sequence. Such work needs a concrete new capability and a
bounded stopping condition.

## Update When

Promote observable behavior to executable evidence under
`../../examples/specification/` first when practical, then update the smallest
matching specification page. Move completed proposal history to
`../reference/implemented-proposals/` and remove it from this catalog.
