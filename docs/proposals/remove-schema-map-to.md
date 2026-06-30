# Remove Schema Map To

Status: proposed

This proposal removes schema-level `map to` clauses from the language surface.
It keeps schemas as external representation contracts and moves value
projection into ordinary Veln functions or explicit codec implementations.

## Problem

`map to` started as a compact way to hide wire-only fields and project decoded
schema fields into an ordinary record shape. The feature has grown into a
second expression and encode-projection language inside `schema` declarations.

That makes schema declarations hard to read and hard to explain:

- `map to Target` changes the decoded helper return shape.
- Generated encode helpers may accept the target shape instead of the
  schema-local shape.
- Whether encode is available depends on implicit reverse projection rules.
- Converter calls need `inverse` annotations to participate in encode.
- Selected mappings add selector coverage, overlap, and target-shape rules.
- Mapping expressions duplicate ordinary function concerns inside schema
  syntax.

The HTTP/2 sans-I/O examples do not need `map to` to express their core
boundary. They use schemas for wire layout and local validation, then use
ordinary values and functions for protocol meaning. That is the clearer model
for the language.

## Proposal

Remove the following source syntax:

```text
map to Target
	field = expression

map to Target when selector
	field = expression

field = expression inverse converter
```

After this change, a schema declaration contains only representation fields,
format selection, field-local predicates, and schema-level validation. It does
not name an internal value type and does not contain mapping assignment lines.

Generated schema helpers decode and encode the schema-local visible record
shape. Representation-only fields such as `ReservedBits(width, value)` remain
validated and omitted from that visible record unless the schema vocabulary
defines a visible representation form for that field kind.

Projection between a schema-local record and a domain value is written as
ordinary source:

```veln
schema HeaderWire
	format binary

	wire_length: UInt16be
	wire_kind: UInt8
end

type Header
	Header {length: Int, kind: Int}
end

fn header_from_wire(wire: {wire_length: Int, wire_kind: Int}) -> Header
	Header {length: wire.wire_length, kind: next_kind(wire.wire_kind)}
end

fn header_to_wire(header: Header) -> {wire_length: Int, wire_kind: Int}
	{wire_length: header.length, wire_kind: previous_kind(header.kind)}
end
```

Codec declarations may still derive direct schema-local decode or encode when
the codec target is the schema-local visible record shape. If a codec's public
target is a different domain type, the codec should use explicit functions or
hand-written implementations that call the generated schema helper and then
project values with ordinary Veln code.

## Scope

The removal covers:

- parser support for schema mapping clauses
- AST, lowered IR, formatter, and editor token handling for mapping clauses
- semantic analysis for mapping target resolution, mapping expressions,
  mapping selection, and inverse projection
- generated decode helper return-shape rewriting through `map to`
- generated encode helper input-shape rewriting through `map to`
- derived codec boundaries that rely on mapped target shapes
- diagnostics whose only purpose is validating schema mapping clauses or
  mapping encode projection
- executable specification examples that exist only to pin mapping behavior

The removal does not cover:

- ordinary `schema` and `pub schema` declarations
- public schema aliases
- field-local `where` predicates
- schema-level `validate` predicates
- exact-width integer, flag, reserved-bit, repeat, byte-view, or dispatch
  representation primitives
- generated helpers over schema-local visible records
- codec declarations that explicitly call helper functions and ordinary
  projection functions

## Migration

Existing schemas with only field declarations need no source change unless
callers relied on a mapped return shape.

Existing schemas with `map to` should be migrated by:

- deleting each `map to` block
- updating direct helper call sites to receive the schema-local visible record
- adding `from_wire` and, when needed, `to_wire` functions for domain values
- replacing `inverse` annotations with explicit inverse functions called from
  encode paths
- replacing selected mappings with ordinary branch logic in the decode or
  encode function

The migration should prefer local, named functions over anonymous inline
projection when the conversion is part of a public codec boundary.

## Diagnostics

After parser removal, `map to` inside a schema is ordinary invalid syntax. The
primary parse diagnostic should report that schema mapping clauses are no
longer accepted and point at the `map` token.

Mapping-specific diagnostics such as selection ambiguity, unsupported mapping
expressions, and mapping encode projection failures should disappear from
current behavior. Historical references may remain only in implemented
proposal records or compatibility notes.

## Specification Work

Implementation should update the executable grammar first, then current
specification prose and examples:

- remove `SchemaMapping`, `SchemaMappingSelector`,
  `SchemaMappingAssignment`, and mapping inverse grammar from
  `source-surface-executable.pl`
- update `source-surface.md` and `execution.md` so schemas return
  schema-local visible records
- remove or rewrite mapping-only examples under `examples/specification/`
- add one executable check that `map to` in a schema is rejected with the new
  parse diagnostic
- update codec examples that used mapped target shapes to call ordinary
  projection functions explicitly

## Compatibility

This is a breaking source change. The project should not keep a compatibility
alias or hidden acceptance path for `map to`; accepting the syntax would
preserve the confusing behavior this proposal removes.

The intended replacement is explicit source code at the codec or helper-call
boundary, where normal name resolution, type checking, effects, tests, and
review apply.
