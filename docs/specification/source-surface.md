# Source Surface

This page routes implemented source syntax. Open
[source-surface-full.md](source-surface-full.md) only when a short route here
is not enough.

## Read First

- Source path derived local module identity, local and external package
  imports, functions, tests, source ADT type declarations, schema
  declarations, public member aliases, canonical `#` comments, `##`
  documentation comments, doctests, ADR-lite metadata, and manifest dependency
  metadata plus `[lib].exports` source-file exports:
  [source-surface-full.md](source-surface-full.md).
- Expression forms, constructors, records, dictionaries, vecs, matches,
  `if` / `else if` / `else` expressions, pipelines, ordinary and variadic
  calls, standard channel calls, zero-argument task spawns, one-context
  `task::spawn_with` calls, and method-call diagnostics:
  [source-surface-full.md](source-surface-full.md).
- Contract predicate grammar:
  [source-surface-full.md](source-surface-full.md).
- Formatter layout and canonical comment spelling:
  [commands.md](commands.md).

## Integer Literals

`Int` literals accept decimal digits, lowercase `0b` plus binary digits, or
lowercase `0x` plus mixed-case hexadecimal digits. All three spellings use the
same nonnegative range through `9223372036854775807` and compare or match by
value. Leading zeroes, radix prefixes, and hexadecimal digit case are retained
by formatting.

Malformed prefixed candidates remain one token. Missing or invalid digits,
uppercase prefixes, separators, prefixed float forms, and out-of-range values
produce one `parse.integer_literal` diagnostic at the failed source fact. The
checked expression and pattern behavior is in
`examples/specification/run/integer-radix-equivalence/`; representative schema
positions are in
`examples/specification/check/integer-radix-schema-positions/`; formatter and
human plus JSON diagnostics are checked by the matching `integer-radix-*`
cases under `examples/specification/`.

## Integer Bitwise Tokens

Expressions accept unary `~` and binary `&`, `|`, `^`, `<<`, `>>`, and
`>>>`. Lexing chooses the longest token, preserving `|>` as pipeline and
distinguishing `>>>`, `>>`, `>=`, and `>`. Adjacent closing angles in nested
generic types remain type delimiters rather than shift expressions.

## Schemas

Top-level `schema Name` and `pub schema Name` declarations are source module
items. A schema body may omit its `format` clause when every field uses
format-neutral type text. Format-neutral generated decode helpers are exposed
only when every field is a recursive format-neutral visible shape made from
scalar leaves, anonymous record fields, `Option<T>`, `List<T>`, `Vec<T>`, and
`Dict<String, T>`. `Result<Ok, Err>` is supported when both payloads are
recursive format-neutral visible shapes. Same-module source ADT fields and
public imported source ADT fields referenced through written `use` paths are
supported when every constructor payload is a recursive format-neutral visible
shape; private imported source ADTs, missing paths, non-ADT targets, and source
ADTs with unsupported payloads remain unsupported helper fields and are
declaration diagnostics.
Format-neutral generated encode helpers are exposed only for schemas without a
`format` clause whose fields are scalar leaves, `Option<scalar>` fields,
`Option<List<scalar>>` fields, `List<scalar>` fields,
`List<Option<scalar>>` fields, `List<Option<List<scalar>>>` fields,
`Vec<scalar>` fields,
`Vec<Option<scalar>>` fields,
`Dict<String, scalar>` fields, `Dict<String, Option<scalar>>` fields,
`Dict<String, List<scalar>>` fields,
`Dict<String, Vec<scalar>>` fields,
`Dict<String, Vec<Option<scalar>>>` fields,
`Option<Dict<String, scalar>>` fields, `Result<Ok, Err>` fields when both
payloads are supported format-neutral encode shapes,
`List<Result<Ok, Err>>`, `Vec<Result<Ok, Err>>`, and
`Dict<String, Result<Ok, Err>>` fields when each result payload is a supported
format-neutral encode shape, or anonymous record fields whose fields are
supported format-neutral encode shapes.
Same-module source ADT fields and public imported source ADT fields referenced
through written `use` paths are also supported when every constructor payload
is a supported format-neutral encode shape. The supported scalar leaves are
`Int`, `Bool`, `Float`, and `String`.
When present, the single `format binary` clause must appear before schema
fields.

Schema field lines contain a field name, `:`, type text, and an optional
field-local `where` predicate. One schema-level `validate` predicate may
appear after fields. Binary schema field vocabulary includes the implemented
exact-width unsigned primitives, lowercase `uint...` fields,
`uint... reserves <value>`
reserved-bit fields, `ReservedBits(width, value)`, `Repeat(count, Payload)`,
canonical repeated fields `[Payload; count]`, direct nested binary schema
fields, recursive anonymous record fields whose leaves are exact-width
unsigned primitives,
`ByteView(length)`, closed dispatch, and extension dispatch forms documented in
[source-surface-full.md](source-surface-full.md) and checked by
`docs/specification/source-surface-executable.pl`. Canonical repeated fields
write the payload field type before `;` and the count expression after it; the
count expression may name an earlier visible count field or use implemented
arithmetic forms over earlier count fields, and the payload may use an
exact-width primitive, a lowercase exact-width primitive, a nested binary
schema, `ByteView(length_field)`, or
`ByteView(left_length - right_length)`.
Direct nested binary schema fields may name an eligible same-module schema or
public imported schema and expose the nested schema-local visible record at
that field.
Anonymous record fields in `format binary` schemas expose a nested
schema-local visible record at that field when every leaf is an implemented
exact-width unsigned primitive. Anonymous records may contain sibling nested
anonymous record fields at the same record level. The checked decode cases are
`examples/specification/run/binary-schema-anonymous-record-decode/`,
`examples/specification/run/binary-schema-nested-anonymous-record-decode/`,
`examples/specification/run/binary-schema-sibling-nested-anonymous-record-decode/`,
and
`examples/specification/run/binary-schema-recursive-anonymous-record-decode/`.
The checked nested truncation JSON cases are
`examples/specification/run/binary-schema-anonymous-record-truncated-json/`,
`examples/specification/run/binary-schema-nested-anonymous-record-truncated-json/`,
`examples/specification/run/binary-schema-sibling-nested-anonymous-record-truncated-json/`,
and
`examples/specification/run/binary-schema-recursive-anonymous-record-truncated-json/`.
The checked encode cases are
`examples/specification/run/binary-schema-anonymous-record-encode/`,
`examples/specification/run/binary-schema-anonymous-record-encode-out-of-range-json/`,
and
`examples/specification/check/binary-schema-anonymous-record-encode-boundary/`.
Legacy `Repeat(count, Payload)` fields accept the same lowercase exact-width
primitive payload spellings that are accepted by canonical repeated-field
syntax. They also accept supported lowercase `uint... reserves <value>`
payloads as representation-only repeated payloads.
Closed and extension dispatch payload cases accept the same lowercase
exact-width `uint...` spelling wherever the compatible upper-case exact-width
primitive payload spelling is accepted. Byte-aligned
lowercase `uint... reserves <value>` payloads are accepted wherever direct
reserved-bit dispatch payloads are supported; direct dispatch payloads also
accept subbyte spellings from `uint1 reserves 0` through
`uint7 reserves 127` when the reserved value fits the declared width.
Binary-only primitive
vocabulary remains gated by `format binary`. In `format binary` schemas,
`veln fmt` writes supported compatibility primitive spellings as the canonical
lowercase schema vocabulary, including direct fields, supported reserved
fields, repeated fields, and dispatch payload field text.
Closed dispatch payload schemas may contain bounded repeated fields whose
payload is an eligible nested binary schema; checked decode coverage is
`examples/specification/run/binary-schema-dispatch-nested-repeat-decode/` and
checked nested truncation output is
`examples/specification/run/binary-schema-dispatch-nested-repeat-truncated-json/`.

Schema declarations return and accept schema-local visible record shapes
through explicit schema operation expressions. The expression
`decode SchemaName from view at base_offset`
accepts eligible binary schemas, `ByteView`, and `ByteOffset` operands and
returns a `DecodeStep<T>` for the schema-local visible record shape. The
expression `encode SchemaName from value` accepts the schema-local visible
record shape for eligible binary schemas and returns
`Result<ByteChunk, EncodeError>`. For format-neutral schemas without a
`format` clause, the same expression accepts and returns the schema-local
visible record shape as `Result<T, String>` without producing binary bytes
when every field is one of the supported format-neutral encode shapes:
scalar leaves, `Option<scalar>`, `Option<List<scalar>>`, `List<scalar>`,
`List<Option<scalar>>`, `List<Option<List<scalar>>>`, `Vec<scalar>`,
`Vec<Option<scalar>>`,
`Dict<String, scalar>`,
`Dict<String, Option<scalar>>`, `Dict<String, List<scalar>>`,
`Dict<String, Vec<scalar>>`,
`Dict<String, Vec<Option<scalar>>>`,
`Option<Dict<String, scalar>>`, `Result<Ok, Err>` when both payloads are
supported format-neutral encode shapes, `List<Result<Ok, Err>>`,
`Vec<Result<Ok, Err>>`, and `Dict<String, Result<Ok, Err>>` when each result
payload is a supported format-neutral encode shape, or anonymous record fields
whose fields are supported format-neutral encode shapes, including those same
result-container shapes, plus same-module source ADT fields and public
imported source ADT fields referenced through written `use` paths when every
constructor payload is a supported format-neutral encode shape.
Qualified public schema paths are accepted when the imported schema or public
schema alias is visible.
The executable coverage is
`examples/specification/run/schema-decode-expression/` and
`examples/specification/run/schema-encode-expression/`. Format-neutral encode
coverage is
`examples/specification/run/format-neutral-schema-scalar-encode/`,
`examples/specification/run/format-neutral-schema-option-scalar-encode/`,
`examples/specification/run/format-neutral-schema-list-scalar-encode/`,
`examples/specification/run/format-neutral-schema-list-option-encode/`,
`examples/specification/run/format-neutral-schema-vec-scalar-encode/`,
`examples/specification/run/format-neutral-schema-nested-vec-scalar-encode/`,
`examples/specification/run/format-neutral-schema-option-vec-encode/`,
`examples/specification/run/format-neutral-schema-dict-scalar-encode/`,
`examples/specification/run/format-neutral-schema-dict-option-scalar-encode/`,
`examples/specification/run/format-neutral-schema-dict-list-scalar-encode/`,
`examples/specification/run/format-neutral-schema-dict-vec-scalar-encode/`,
`examples/specification/run/format-neutral-schema-option-dict-encode/`,
`examples/specification/run/format-neutral-schema-option-list-encode/`,
`examples/specification/run/format-neutral-schema-nested-container-encode/`,
`examples/specification/run/format-neutral-schema-result-scalar-encode/`,
`examples/specification/run/format-neutral-schema-result-option-encode/`,
`examples/specification/run/format-neutral-schema-recursive-result-encode/`,
`examples/specification/run/format-neutral-schema-result-container-encode/`,
and
`examples/specification/run/format-neutral-schema-source-adt-encode/`. The
HTTP/2 frame header schema boundary is checked through explicit decode
operations under
`examples/specification/run/binary-schema-frame-header-decode/`,
`examples/specification/run/binary-schema-frame-header-reserved-human/`,
`examples/specification/run/binary-schema-frame-header-reserved-json/`,
`examples/specification/run/binary-schema-frame-header-truncated-human/`, and
`examples/specification/run/binary-schema-frame-header-truncated-json/`.
Encode expression diagnostics are checked by
`examples/specification/check/schema-encode-expression-diagnostics/` and
runtime value failures by
`examples/specification/run/schema-encode-expression-unrepresentable-human/`
and
`examples/specification/run/schema-encode-expression-unrepresentable-json/`.
Projection into domain records is ordinary source code at the caller or
schema-operation boundary. Generated schema helper names are compatibility
implementation details, not the documented source API for applying schemas.
The checked format-neutral generated helper cases are
`examples/specification/run/format-neutral-schema-decode/`,
`examples/specification/run/format-neutral-schema-option-list-decode/`,
`examples/specification/run/format-neutral-schema-nested-option-list-decode/`,
`examples/specification/run/format-neutral-schema-recursive-containers-decode/`,
`examples/specification/run/format-neutral-schema-result-decode/`,
`examples/specification/run/format-neutral-schema-source-adt-decode/`,
`examples/specification/run/format-neutral-schema-scalar-encode/`,
`examples/specification/run/format-neutral-schema-option-scalar-encode/`,
`examples/specification/run/format-neutral-schema-list-scalar-encode/`,
`examples/specification/run/format-neutral-schema-list-option-encode/`,
`examples/specification/run/format-neutral-schema-vec-scalar-encode/`,
`examples/specification/run/format-neutral-schema-nested-vec-scalar-encode/`,
`examples/specification/run/format-neutral-schema-option-vec-encode/`,
`examples/specification/run/format-neutral-schema-dict-scalar-encode/`,
`examples/specification/run/format-neutral-schema-dict-option-scalar-encode/`,
`examples/specification/run/format-neutral-schema-dict-list-scalar-encode/`,
`examples/specification/run/format-neutral-schema-dict-vec-scalar-encode/`,
`examples/specification/run/format-neutral-schema-option-dict-encode/`,
`examples/specification/run/format-neutral-schema-option-list-encode/`,
`examples/specification/run/format-neutral-schema-nested-container-encode/`,
`examples/specification/run/format-neutral-schema-result-scalar-encode/`,
`examples/specification/run/format-neutral-schema-result-option-encode/`,
`examples/specification/run/format-neutral-schema-recursive-result-encode/`,
`examples/specification/run/format-neutral-schema-result-container-encode/`,
`examples/specification/run/format-neutral-schema-source-adt-encode/`,
`examples/specification/run/format-neutral-schema-dict-vec-option-encode/`,
`examples/specification/check/format-neutral-schema-container-encode-boundary/`,
`examples/specification/check/format-neutral-schema-recursive-result-encode-boundary/`,
`examples/specification/check/format-neutral-schema-result-container-encode-fields/`,
`examples/specification/check/format-neutral-schema-result-container-encode-boundary/`,
`examples/specification/check/format-neutral-schema-list-scalar-encode-boundary/`,
`examples/specification/check/format-neutral-schema-list-option-encode-boundary/`,
`examples/specification/check/format-neutral-schema-vec-scalar-encode-boundary/`,
`examples/specification/check/format-neutral-schema-dict-scalar-encode-boundary/`,
`examples/specification/check/format-neutral-schema-dict-vec-encode-boundary/`,
`examples/specification/check/format-neutral-schema-option-dict-encode-boundary/`,
`examples/specification/check/format-neutral-schema-vec-fields/`,
`examples/specification/check/format-neutral-schema-source-adt-fields/`,
`examples/specification/check/format-neutral-schema-source-adt-helper-diagnostics/`,
`examples/specification/check/format-neutral-schema-source-adt-helper-diagnostics-human/`,
`examples/specification/check/format-neutral-schema-container-encode-boundary-human/`,
and
`examples/specification/check/format-neutral-schema-decode-helper-diagnostics/`.
Schema-level `map to` clauses, selected
schema mappings, mapping assignments, and `inverse` projection annotations are
not accepted source syntax.

## Codecs

Top-level `codec Name for Schema directions...` and
`pub codec Name for Schema directions...` declarations are not accepted source
syntax. The parser reports `parse.codec_declaration_removed` at the `codec`
token and directs source toward ordinary functions plus explicit
`decode Schema from view at base_offset` and `encode Schema from value`
expressions.

## Diagnostics

The parser rejects schema-level `map to` with
`parse.schema_mapping_removed` at the `map` token. The checked cases cover
plain mapping clauses, selected mapping clauses, and inverse projection
annotations under `examples/specification/check/schema-map-to-*-rejected/`.
Mapping-only semantic and runtime diagnostics are not current behavior. The
parser rejects top-level codec declarations with
`parse.codec_declaration_removed`. Current schema diagnostics cover
format placement, primitive kind checks, field-local and schema-level
validation predicates, dispatch payload eligibility, explicit schema decode
expression schema-path resolution, and helper availability. Binary schema
field references for repeat counts, `ByteView` lengths and multiple
constraints, dispatch tags, and extension-dispatch tags and lengths must name
earlier decoded visible `Int` fields in the same schema. Invalid references
use `schema.repeat_reference`, `schema.byte_view_reference`, or
`schema.dispatch_reference`, with checked JSON and human output under
`examples/specification/check/binary-schema-field-reference-diagnostics/` and
`examples/specification/check/binary-schema-field-reference-human/`.

## Read When

- Updating parser behavior, AST source shape, source metadata, declaration
  rules, or formatter output.
- Checking whether a syntax feature is implemented rather than proposed.
- Aligning examples, diagnostics, or command behavior with accepted source
  syntax.

## Skip Unless Needed

- Do not read proposal or phase history before this page and the relevant
  section of [source-surface-full.md](source-surface-full.md).
- Use [source-decisions.md](source-decisions.md) only when rationale is needed
  after the implemented source behavior is clear.
