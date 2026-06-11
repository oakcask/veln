# Schema Declaration Surface

Status: proposed

This proposal defines the source syntax needed to declare schemas as external
representation boundaries. It is a prerequisite for the HTTP/2 binary schema
design driver because that driver needs a source-visible way to describe frame
header layout before codec execution or protocol state rules can be tested.

## Problem

Current source syntax has functions, tests, ADT type declarations, records,
contracts, effects, imports, and public aliases. It does not have a top-level
declaration for an external representation boundary.

The HTTP/2 design driver needs a declaration that can say:

- a field is read from bytes rather than from an internal Veln value
- a field has a fixed external width
- a field is validated at the schema boundary
- a field may map into an independently declared Veln type
- a schema reports structural failures with field paths and byte positions

Without a schema declaration, binary protocol examples must encode external
layout in ordinary functions, which hides the boundary the driver is meant to
exercise.

## Scope

Define source support for:

- top-level `schema` declarations
- named schema fields
- field type annotations that may name schema primitives
- field-local validation clauses
- mapping from schema fields to Veln values
- schema visibility and module ownership rules
- parser, AST, formatter, editor token, and documentation behavior

## Discussion Result: Codec Binding Direction

Schema declarations should not name the codec declarations that decode or
encode them. A schema is the reusable external representation contract; it
remains meaningful when a module exposes a decoder, an encoder, both
directions, fixture-only helpers, or no executable codec yet.

The binding should point from a codec declaration to the schema it implements.
That keeps schema declarations free of executable API ownership and lets the
codec boundary decide direction, readiness variants, consumed byte counts,
offset handling, imports, and exported names. A schema may still import or
select field vocabularies for its representation format, but that selection is
not the same thing as naming executable codec entry points.

This means a schema can be imported and referenced as a boundary contract on
its own. Modules expose executable decoding or encoding by exporting codec
declarations that cite the schema, rather than by adding codec names inside the
schema body.

## Discussion Result: Schema Value Mapping

Schema field values should map into independently declared source records and
ADTs through an explicit mapping clause. A schema does not implicitly publish a
record type just because it names fields, and importing a schema does not make
its schema-local field names available as ordinary source bindings.

The mapping clause should name the target value shape and assign schema-local
fields to the target's record fields or ADT constructor payload fields. Fields
whose names begin with `_` are omitted from the produced value unless the
mapping explicitly includes them. This keeps reserved bits and other
representation-only facts available for validation and diagnostics without
turning them into protocol-domain data by accident.

Mapping is checked after schema field validation and before the decoded value
is returned by a codec. The checker should resolve target record fields and ADT
constructors through the normal source module rules, reject missing or
duplicate assignments, and require each assigned schema value to type check
against the target field. Schema primitive values may use the primitive's
declared ordinary value type, such as `Int` for exact-width unsigned fields, or
a field-local representation conversion when the schema vocabulary defines one.

Mapping expressions should stay structural in the first surface: field
selection, record construction, ADT construction, and schema-declared
representation conversions. Arbitrary function calls, runtime settings, stream
state, and recovery behavior belong in explicit codec functions rather than in
schema mapping.

## Discussion Result: Top-Level Schema Declarations

`schema` should be a normal top-level declaration beside declarations such as
`type`, `fn`, and `test`, not a modifier on a type declaration and not a
specialized `codec schema` form.

The declaration name owns an external representation boundary. Its body owns
schema-local fields, validation clauses, mapping clauses, and the
format-specific vocabulary selected by its `format` clause. It does not create
an ordinary Veln value type by itself, and it does not imply executable decode
or encode APIs. Codecs cite schemas from their own declarations when a module
wants to expose execution.

This keeps the parser, formatter, editor support, documentation, and module
item model direct: a source file contains a schema item with a stable name and
span. It also keeps schemas reusable across generated codecs, hand-written
codecs, fixtures, documentation, and diagnostic tests without forcing every
schema to commit to one executable direction.

## Discussion Result: Schema Imports And References

Schema visibility should follow the ordinary source module boundary. A private
schema is visible only in its declaring module. A `pub schema` declaration is
part of the declaring module's public API when the module's source file is
listed by the package manifest's `[lib].exports`.

References to schemas should use schema-aware name resolution rather than value
resolution. Codecs, schema composition, fixture helpers, and documentation
examples may reference a schema by bare name inside the declaring module. From
another module, they reference public schemas through the written `use` module
path, such as `http2::FrameHeader`. A `use` declaration does not re-export the
schema from the importing module.

Importing or referencing a schema imports only the schema item. It does not
import schema-local field names as ordinary bindings, expose a generated record
type, or make any decoder or encoder available. Executable APIs remain owned
by public codec declarations that explicitly cite the schema.

The first surface should not add schema member aliases. Facade modules can
publish selected executable codecs, and a later alias proposal can add schema
aliases with explicit wrong-kind diagnostics if real packages need that API
shape.

## Discussion Result: Field Validation Spelling

Schema field validation should be spelled as field-local `where` clauses:

```text
schema PaddedPayload
  format binary

  length: UInt24be
  padding_length: UInt8 where padding_length <= length
  payload: ByteView(length - padding_length)
end
```

A `where` clause belongs to the field it follows. It is checked after that
field has been decoded and before later fields may reference the validated
value. The predicate may name the current field and fields decoded earlier in
the same schema. It must not name later fields, ordinary source bindings,
runtime settings, connection state, stream state, or imported functions.

The predicate language should reuse the familiar comparison, boolean, literal,
field-reference, and arithmetic operators from contract predicates, but with a
schema-local resolver and without `require`, `ensure`, or `invariant`
keywords. This keeps representation checks readable without making schema
validation part of the ordinary function contract system.

A failed `where` clause is a schema structural failure at the owning field
path. Diagnostics should report the field path, byte offset, failed predicate,
and relevant decoded field values through structured data and related notes.
Protocol-state limits that depend on negotiated settings remain explicit
codec or protocol-core checks rather than schema `where` clauses.

## Discussion Result: Format Vocabulary Selection

Schema declarations should select their external representation vocabulary
inside the schema body with an explicit format clause, rather than by using a
specialized declaration keyword such as `codec schema`.

The first clause should be `format binary`. It makes binary schema primitives
such as exact-width integers, reserved bits, byte ranges, and dispatch forms
available only in that schema's field vocabulary. The clause does not import
ordinary source values, does not create executable codec APIs, and does not
change the module visibility of the schema item.

```text
schema Http2FrameHeader
  format binary

  length: UInt24be
  kind: UInt8 as FrameKind
  flags: UInt8
  _stream_reserved: ReservedBits(1, 0)
  stream_id: UInt31be
end
```

The parser should require the format clause before format-specific fields or
validation forms are used. A schema with no format clause may contain only the
format-neutral surface accepted by the schema declaration proposal; binary
primitives in that context are wrong-kind schema diagnostics, not ordinary
unresolved value names.

Future formats can add new `format <name>` clauses with their own field
vocabularies. A single schema uses one format in the first surface. Shared
ordinary Veln types, records, ADTs, and mapping targets are still referenced
through normal module imports, while representation primitives remain owned by
the selected schema format.

## Non-Goals

- Do not define the complete binary primitive vocabulary here.
- Do not implement HTTP/2 protocol state rules here.
- Do not require a network runtime.
- Do not treat schemas as aliases for internal Veln types.

## Completion Criteria

- The accepted grammar includes schema declarations.
- Parser, AST, formatter, and editor support understand schema declarations.
- Examples show schema declarations as boundary contracts, not ordinary types.
- Diagnostics distinguish malformed schema syntax from failed schema
  validation.
- The HTTP/2 design driver can express its frame header boundary without using
  placeholder text syntax.
