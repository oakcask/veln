# Schema Declaration Surface

Status: proposed

This proposal generalizes the existing schema field surface to composition
behavior that is not current behavior. Start with
[Source Surface](../specification/source-surface.md) and
[Execution](../specification/execution.md) for implemented declarations,
explicit schema operations, aliases, nested binary fields, visibility,
validation, and helper behavior.

## Problem

Veln schemas already provide top-level external-representation declarations,
format-neutral recursive visible-shape helpers, binary nested-schema fields,
explicit decode and encode operations, public schema aliases, documentation
references, and fixture metadata references. They do not provide one explicit
rule that treats the existing field type position as a schema-aware composition
reference across the format-neutral and binary surfaces. References from a
later field into an earlier nested schema value are also not a uniform part of
length, repeat, dispatch, predicate, and validation expressions.

## Composition Surface

Composition reuses the existing field syntax:

```veln
schema Envelope
  metadata: common::Metadata
  payload: String
end
```

Binary composition keeps the current nested-schema spelling and participates in
representation order:

```veln
schema Frame
  format binary

  header: wire::FrameHeader
  payload: ByteView(header.length)
end
```

No new keyword or declaration-body grammar is introduced. When the right side
of `<field>: <reference>` resolves to a schema, the field name is the stable
local composition binding and becomes one nested field in the containing
schema-local visible record. Composition never injects the target's fields as
unqualified names.

## Composition Semantics

- Decode consumes the target schema at the field position and stores its
  schema-local visible record under the binding. Encode reads that nested
  record and emits the target representation at the same position.
- A format-neutral schema may compose only a format-neutral schema. A binary
  schema may compose only a binary schema. Other format combinations are
  rejected at the composition reference.
- An earlier composition binding may be used by later length, repeat,
  dispatch, field predicate, and schema validation expressions through an
  explicit path such as `header.length`. A later binding is a forward
  reference and is rejected.
- Target validation runs at the nested boundary before the containing schema
  commits its next state. Containing-schema validation runs after all composed
  and ordinary fields are available.
- Decode and encode eligibility are checked independently. If the target lacks
  a helper in the required direction, the containing schema receives a focused
  composition eligibility diagnostic and emits no helper in that direction.
- Diagnostics preserve the containing schema and composition binding before
  the target schema and target field path. A nested failure exposes no partial
  containing value or encoded bytes.

## Schema-Aware Resolution

- Binary primitives and schema combinators continue to use their existing
  grammar before nominal-path lookup. Format-neutral structural types and
  containers continue to use their existing type grammar.
- For a nominal field reference, semantic analysis checks the ordinary type
  namespace and schema namespace independently. A unique schema result selects
  composition; a unique type result keeps the current ordinary field behavior.
- If the same written path resolves as both an ordinary type and a schema, the
  field is ambiguous and receives a focused diagnostic. The declaration must
  use a non-conflicting type or schema alias; composition does not gain a
  keyword-based precedence rule.
- A bare schema path resolves a private or public schema or schema alias in the
  same module. A qualified path requires the matching written `use` path and
  resolves only a public schema or public schema alias in the imported module.
- Alias chains resolve transitively through the existing schema namespace.
  Missing, private, cyclic, and wrong-kind targets are rejected at the written
  path with distinct reasons.
- Composition bindings occupy only the containing schema's field namespace.
  They do not import target field names, create ordinary value or source type
  bindings, or manufacture generated helper aliases.
- Direct and indirect schema-composition cycles are rejected before helper
  eligibility or typed IR construction.

## Future Binary Shape Admission

Binary field shapes outside current generated-helper eligibility are not scope
of this proposal. Add one only through a separate focused proposal that names a
concrete protocol representation, decode and encode value shape, diagnostic
precedence, and stopping condition. A new width, operator combination,
reserved-bit layout, container nesting, or dispatch variant is not a target
merely because it is the next same-shaped case.

## Non-Goals

- Do not reopen implemented format-neutral decode or encode eligibility.
- Do not add another binary primitive or generated-helper field family.
- Do not restore schema-level `map to` clauses or source-level codec
  declarations.
- Do not define arbitrary bitstreams, signed integers, floating-point binary
  encodings, variable-length integers, or text encodings without a concrete
  protocol requirement.
- Do not add HTTP/2 state rules or require a network runtime.
- Do not treat schemas as aliases for internal Veln types.

## Completion Criteria

- Semantic analysis, typed IR, and runtime lowering recognize schema references
  in the existing field type position for format-neutral and binary schemas.
- Parser and formatter regression cases confirm that composition adds no new
  keyword or source grammar.
- Executable cases cover local private and public targets, imported public
  schemas and aliases, multiple ordered compositions, explicit nested field
  references, decode and encode, validation, and nested diagnostic paths.
- Diagnostic cases cover missing, private, wrong-kind, cyclic,
  type/schema ambiguity, format-incompatible, forward-reference,
  duplicate-binding, and direction-specific ineligible targets.
- No target field becomes visible without its composition binding, and no
  ordinary value, source type, or generated helper alias is introduced.
- Implemented slices are promoted to `docs/specification/` and
  `examples/specification/`, then archived under
  `docs/reference/implemented-proposals/` instead of accumulating here.
