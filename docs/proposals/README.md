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

- [HTTP/2 Standard Library Completion and Fixture Retirement](http2-sans-io-protocol-core.md):
  complete HPACK and sans-I/O core ownership, migrate every monolithic fixture
  assertion, and remove the broad protocol-core case only after its deletion
  gate is satisfied.
- [Network Effect Integration Boundary](network-effect-integration-boundary.md):
  external production socket ownership beyond the loopback harness.
- [Schema Declaration Surface](schema-declaration-surface.md): generalize the
  existing field syntax to named schema composition with schema-aware
  resolution.

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
