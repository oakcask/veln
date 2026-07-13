# Schema Declaration Surface

Status: proposed

This proposal tracks only schema declaration work that is not current behavior.
Start with [Source Surface](../specification/source-surface.md) and
[Execution](../specification/execution.md) for implemented syntax, visibility,
validation, and helper behavior. Use
[Recursive Format-Neutral Schema Encode Shapes](../reference/implemented-proposals/recursive-format-neutral-schema-encode-shapes.md)
for the completed recursive encode eligibility decision and its evidence.

## Problem

Veln schemas already provide top-level external-representation declarations,
format-neutral recursive visible-shape helpers, and a broad set of binary
helper slices. Two gaps remain:

- some binary field shapes cannot yet synthesize generated runtime helpers
- later schema-composition surfaces do not yet resolve schemas

Keeping these gaps explicit prevents completed source, diagnostic, and helper
behavior from being mistaken for proposal work.

## Scope

### Remaining Binary Helpers

Extend generated binary schema decode and encode bindings only when a concrete
field shape falls outside the implemented exact-width unsigned integer,
representation-only reserved-bit, direct nested schema, anonymous record,
bounded repeat, length-bounded byte view, closed dispatch, and extension
dispatch slices.

Each extension must preserve schema-local visibility, declaration-order
validation, structured field paths and byte positions, and representability
failure precedence. New primitive families require their own focused design;
this proposal does not imply an unbounded sequence of widths or encodings.

### Later Schema Composition

Define schema-aware references for future composition surfaces beyond current
explicit schema operations, public schema member aliases, documentation
comments, binary fixture metadata, and the removed codec compatibility
surface.

A composition design must keep schema lookup distinct from ordinary value and
type lookup. It must not implicitly import field names, create ordinary value
types, or manufacture generated helper aliases.

## Non-Goals

- Do not reopen implemented format-neutral decode or encode eligibility.
- Do not restore schema-level `map to` clauses or source-level codec
  declarations.
- Do not define arbitrary bitstreams, signed integers, floating-point binary
  encodings, variable-length integers, or text encodings without a concrete
  protocol requirement.
- Do not add HTTP/2 state rules or require a network runtime.
- Do not treat schemas as aliases for internal Veln types.

## Completion Criteria

- Every remaining binary field shape selected for implementation has generated
  decode and encode coverage, focused diagnostics, and executable
  specification evidence.
- The later schema-composition surface has explicit syntax and schema-aware
  visibility and resolution rules.
- Implemented slices are promoted to `docs/specification/` and
  `examples/specification/`, then archived under
  `docs/reference/implemented-proposals/` instead of accumulating here.
