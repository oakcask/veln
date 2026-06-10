# Binary Fixture Helpers

Status: proposed

This proposal defines fixture helpers for binary examples and tests. It is a
prerequisite for the HTTP/2 binary schema design driver because frame fixtures
need compact, reviewable source data and stable expected output.

## Problem

Binary protocol tests are difficult to review when byte arrays are spelled as
long lists of integers. HTTP/2 examples need valid and invalid frame fixtures,
truncated input cases, unknown frame type cases, and expected encoded output.

The repository already uses executable examples and checked fixtures as primary
specification evidence where practical. Binary schema work needs equivalent
fixture support.

## Scope

Define support for:

- hex-to-byte fixture helpers
- named binary fixture records
- expected consumed byte counts
- expected output chunks
- truncated input cases
- invalid field cases
- fixture diagnostics that can match byte offsets and field paths

## Required Design Decisions

The proposal must resolve:

- whether binary fixtures live in Veln source, TOML cases, or both
- how hex text is validated and reported
- how fixture helpers avoid hiding byte offsets
- how command output should render byte chunks
- how examples share fixtures without turning them into production APIs

## Non-Goals

- Do not define schema syntax.
- Do not define the full HTTP/2 conformance suite.
- Do not require external test data.
- Do not treat fixture helpers as general-purpose binary serialization APIs.

## Completion Criteria

- Examples include compact valid and invalid binary fixtures.
- Hex parsing diagnostics are stable and distinct from codec diagnostics.
- Test cases can assert byte offsets, field paths, consumed counts, and output
  chunks.
- The HTTP/2 design driver can add frame fixtures without unreadable byte-array
  noise.
