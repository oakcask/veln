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

- [Parallel Test Execution](parallel-test-execution.md): use bounded,
  CPU-aware case scheduling while preserving deterministic reports, static
  gates, and a serial compatibility route.
- [HTTP/2 Standard Library Completion and Fixture Retirement](http2-sans-io-protocol-core.md):
  complete sans-I/O core ownership, migrate the remaining monolithic fixture
  assertions, and remove the broad protocol-core case only after its deletion
  gate is satisfied; production HPACK encoding and decoding are complete.

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
