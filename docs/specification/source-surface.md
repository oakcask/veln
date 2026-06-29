# Source Surface

This is the routing page for implemented source syntax. Use it to choose the
smallest section to read before opening the full grammar notes.

## Read First

- Source path derived local module identity, local and external package
  imports, functions, tests, source ADT type declarations, schema and codec
  declarations, public member aliases, canonical
  `#` comments, `##` documentation comments, doctests, ADR-lite metadata, and
  manifest dependency metadata plus `[lib].exports` source-file exports:
  [source-surface-full.md](source-surface-full.md).
  Use [commands.md](commands.md) for formatter layout and canonical comment
  spelling behavior.
- Doctest fence metadata, `runtime=contract`, `runtime=ensure`, and
  `runtime=result` expectations, expected-output fences, `> ` hidden setup,
  visible hash comments inside doctests, and negative examples:
  [source-surface-full.md#documentation-comments-and-doctests](source-surface-full.md#documentation-comments-and-doctests).
- Expression forms, constructors, records, dictionaries, vecs, matches,
  `if` / `else if` / `else` expressions, pipelines, ordinary and variadic
  calls, standard channel calls, zero-argument task spawns, one-context
  `task::spawn_with` calls, and method-call diagnostics:
  [source-surface-full.md](source-surface-full.md#expressions).
  `if` expressions require a final `else` and `end`; parse recovery covers
  missing `else`, missing `end`, missing `else if` conditions, and malformed
  branch bodies.
- Function declaration parameters may use the final-only variadic spelling
  `name: ...T`; ordinary type positions reject `...T`:
  [source-surface-full.md#grammar](source-surface-full.md#grammar).
- Contract predicate grammar:
  [source-surface-full.md](source-surface-full.md#contract-predicates).

## Read When

- Updating parser behavior, AST source shape, source metadata, or declaration
  rules.
- Checking whether a syntax feature is implemented rather than proposed.
- Aligning examples, diagnostics, or command behavior with accepted source
  syntax.

## Skip Unless Needed

- Do not read proposal or phase history before this page and the relevant
  section of [source-surface-full.md](source-surface-full.md).
- Use [source-decisions.md](source-decisions.md) only when rationale is needed
  after the implemented source behavior is clear.

## Grammar

See [source-surface-full.md#grammar](source-surface-full.md#grammar).

Top-level `schema Name` and `pub schema Name` declarations are implemented as
source module items. The implemented schema body slice requires a single
`format binary` clause before schema fields. Schema field lines contain a field
name, `:`, type text, and an optional field-local `where` predicate. In binary
schemas, `UInt1` through `UInt8`, `UInt16be`, `UInt16le`, `UInt24be`,
`UInt24le`, `UInt31be`, `UInt31le`, `UInt32be`, `UInt32le`, `UInt40be`,
`UInt40le`, `UInt48be`, `UInt48le`, `UInt56be`, `UInt56le`, `UInt64be`,
`UInt64le`, and
`ReservedBits(width, value)` are accepted as schema primitives. `Flag8`,
`Flag16be`, `Flag16le`, `Flag24be`, `Flag24le`, `Flag32be`, `Flag32le`,
`Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`, `Flag56be`, `Flag56le`,
`Flag64be`, and `Flag64le` are
accepted as opt-in visible flag bitset fields; they decode and encode through
source-visible `Flag8(bits: Int)`, `Flag16be(bits: Int)`,
`Flag16le(bits: Int)`, `Flag24be(bits: Int)`, `Flag24le(bits: Int)`,
`Flag32be(bits: Int)`, `Flag32le(bits: Int)`, `Flag40be(bits: Int)`,
`Flag40le(bits: Int)`, `Flag48be(bits: Int)`, `Flag48le(bits: Int)`,
`Flag56be(bits: Int)`, `Flag56le(bits: Int)`, `Flag64be(bits: Int)`, and
`Flag64le(bits: Int)` value types instead of the raw `Int` used by `UInt8`,
`UInt16be`, `UInt16le`, `UInt24be`, `UInt24le`, `UInt32be`, `UInt32le`,
`UInt40be`, `UInt40le`, `UInt48be`, `UInt48le`, `UInt56be`, `UInt56le`,
`UInt64be`, and `UInt64le`.
Source-visible checked helpers read and set `Flag8` bit indexes `0` through
`7`, `Flag16be` and `Flag16le` bit indexes `0` through `15`, `Flag24be` and
`Flag24le` bit indexes `0` through `23`, `Flag32be` and `Flag32le` bit
indexes `0` through `31`, `Flag40be` and `Flag40le` bit indexes `0` through
`39`, `Flag48be` and `Flag48le` bit indexes `0` through `47`, `Flag56be` and
`Flag56le` bit indexes `0` through `55`, and `Flag64be` and `Flag64le` bit
indexes `0` through `63`; indexes
outside each helper's range return
`Result` failures. Source-visible raw-bit helpers expose the wrapped integer
bits and construct `Flag8`, `Flag16be`, `Flag16le`, `Flag24be`, `Flag24le`,
`Flag32be`, `Flag32le`, `Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`,
`Flag56be`, `Flag56le`, `Flag64be`, or `Flag64le`
values only when the supplied integer is inside the corresponding one-byte,
two-byte, three-byte, four-byte, five-byte, six-byte, seven-byte, or
eight-byte range.
`ReservedBits` arguments must be literal
non-negative integers. `ReservedBits(width, value)` may start a supported
reserved prefix group followed by two visible sub-byte or byte-width `UIntN`
fields whose widths complete one byte or the same two-byte, three-byte,
four-byte, five-byte, six-byte, seven-byte, or eight-byte big-endian storage
unit; the five-byte form accepts reserved prefix width thirty-three, the
six-byte form accepts reserved prefix width forty-one, the seven-byte form
accepts reserved prefix width forty-nine, and the eight-byte form accepts
reserved prefix width fifty-seven when the two visible fields complete the
remaining bits. `ReservedBits(15, value)` may also be followed immediately by
`UInt1` when the two fields complete the same two-byte big-endian storage
unit. Two visible `UIntN` fields may also be followed by a
non-byte-aligned `ReservedBits(width, value)` suffix when the second visible
field is `UInt8` and all three widths complete the same two-byte big-endian
storage unit. A single visible `UIntN` field may also be followed by a
non-byte-aligned `ReservedBits(width, value)` suffix when the two widths
complete one byte or the same two-byte, three-byte, four-byte, five-byte,
six-byte, seven-byte, or eight-byte big-endian storage unit.
`Repeat(count_field, Payload)` is accepted as a
bounded repeated field when `count_field` names a previously decoded visible
`Int` field in the same binary schema. `Repeat(left_count - right_count,
Payload)`, `Repeat(left_count + right_count, Payload)`,
`Repeat(left_count * right_count, Payload)`, and
`Repeat(left_count / right_count, Payload)` are accepted when both operands
name earlier visible `Int` fields in the same binary schema.
`Payload` is either an implemented byte-aligned
exact-width unsigned primitive, an eligible nested binary schema payload, or
`ByteView(length_field)` when `length_field` is another earlier visible `Int`
field in the same schema. Length-bounded `ByteView(length_field)`,
`ByteView(left_length - right_length)`,
`ByteView(left_length + right_length)`,
`ByteView(left_length * right_length)`, and
`ByteView(left_length / right_length)` payload fields are accepted when every
length operand names an earlier visible `Int` field in the same binary schema.
A repeated primitive field decodes and encodes as `List<Int>`; a repeated
nested schema field decodes and encodes as a list of the nested schema's
decoded record shape; and a repeated `ByteView` field decodes and encodes as
`List<ByteView>`. Missing, forward, or non-`Int` repeat count references
report `schema.repeat_reference`; missing, forward, or non-`Int` byte-view
length references report
`schema.byte_view_reference`. The narrow closed tag-dispatch field types
`Dispatch(tag_field, tag => Payload, ...)` and
`Dispatch(tag_field, length_field, tag => Payload, ...)` are accepted when
`tag_field` and any `length_field` name previously decoded visible `Int`
fields and each case payload is either one of the implemented exact-width
unsigned binary primitives or an eligible nested binary schema payload.
Nested payload schema names must resolve to an earlier same-module binary
schema item or a public imported binary schema named through a written `use`
path, and the named schema must itself be eligible for the generated binary
schema helper path, including supported representation-only `ReservedBits`
layouts and length-bounded `ByteView(length_field)` or
`ByteView(left_length + right_length)` fields whose length operands name
earlier visible `Int` fields in that nested schema. A same-module recursive
dispatch case may name the enclosing schema recursively, a same-module
dispatch case may name a separate eligible recursive payload schema, and a
public imported recursive payload schema may be named through a written `use`
path, only in the length-bounded form when selected
`map to Target when tag_field == literal` clauses cover every case and all
clauses resolve to one record shape, with at least one non-recursive case as
the base case. A length-bounded parent dispatch field without selected
mappings may also name an earlier same-module recursive payload schema or a
public imported recursive payload schema through a written `use` path when
that payload schema has the bounded recursive helper, the parent has at least
one non-recursive primitive case, and the parent requires only generated
decode helper support. The extension-tolerant field type
`ExtensionDispatch(tag_field, length_field, tag => Payload, ...)` is accepted
when both referenced fields were decoded earlier in the same schema as visible
`Int` fields. Its known cases use the same payload vocabulary, and its unknown
cases preserve a bounded raw payload selected by `length_field`. These
primitive names are representation-local field vocabulary, not ordinary source
types or values.
One schema-level `validate` predicate may appear after binary schema fields.
It uses the same predicate syntax as field-local `where` clauses, but may
reference only `Int` fields decoded by the same schema helper. Unknown field
names, non-`Int` decoded fields, ordinary source bindings, and additional
schema-level validations are rejected through schema validation diagnostics.
Dispatch reference diagnostics report `schema.dispatch_reference` when the tag
or length field is missing, forward, or not an `Int`-decoded schema field.
Nested dispatch payload diagnostics report `schema.dispatch_payload` when a
payload name is missing, resolves to a non-schema item, names a private
imported schema, names a non-binary schema, refers forward, uses an unbounded
or otherwise ineligible recursive form, lacks the generated decode helper
required by a parent decode helper, uses a field-reference-ineligible
`ByteView` payload layout, or decodes to an incompatible payload shape. Parent
recursive payload diagnostics name the failed recursive-helper fact, including
missing length-bounded dispatch, missing primitive base case for unmapped
decode-only parents, or missing selected mapping coverage for encode-capable
parents. Parent decode helpers, generated decode-step helpers, and
`derive decode` require
only the nested schema's decode helper; they do not require the nested schema
to expose a generated encode helper. Generated encode helpers and
`derive encode` still require nested dispatch payload schemas to expose a
generated encode helper. Helper-slice payload diagnostics name the expected
generated decode and encode helpers in structured fields and keep the payload
schema declaration in related notes. When the nested schema has a field layout
that prevents helper exposure, the diagnostic details and related notes also
name the nested schema field path, the unavailable helper directions, and the
specific unsupported `ReservedBits`, `ByteView`, or mapping-projection layout
fact. Closed dispatch
cases with mixed
primitive and
nested payload shapes are accepted only at an eligible selected mapping
boundary where every `map to Target when tag_field == literal` selector uses
the dispatch tag field, covers a distinct dispatch case, and type-checks that
branch against the payload shape selected by the literal. Extension-tolerant
recursive dispatch uses the same selected mapping boundary for known cases and
still preserves unknown tags as bounded raw payloads. Other mixed payload
dispatch shapes keep the `schema.dispatch_payload` rejection. The checked
field-reference
diagnostics case is
`../../examples/specification/check/binary-schema-field-reference-diagnostics/`;
the checked dispatch payload diagnostics case is
`../../examples/specification/check/binary-schema-dispatch-payload-diagnostics/`;
the checked imported recursive payload acceptance case is
`../../examples/specification/check/binary-schema-imported-recursive-dispatch-payload-accepted/`;
the checked recursive payload diagnostics case is
`../../examples/specification/check/binary-schema-recursive-dispatch-payload-diagnostics/`;
the checked same-module recursive decode-only payload example is
`../../examples/specification/run/binary-schema-same-module-recursive-dispatch-decode/`;
the checked additive nested `ByteView` dispatch payload examples are
`../../examples/specification/run/binary-schema-dispatch-nested-byteview-add-decode/`
and
`../../examples/specification/run/binary-schema-dispatch-nested-byteview-add-encode/`;
the checked product-sized nested `ByteView` dispatch payload examples are
`../../examples/specification/run/binary-schema-dispatch-nested-byteview-product-decode/`
and
`../../examples/specification/run/binary-schema-dispatch-nested-byteview-product-encode/`;
the checked helper-eligibility detail cases are
`../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-diagnostics/`
and
`../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-human/`.
A schema may end with
structural `map to Target` clauses whose assignment lines use
`target_field = expression` to map schema-local fields into an ordinary
source value shape. The implemented mapping expression slice supports
schema-local field references, including supported `ReservedBits(width,
value)` fields when an assignment explicitly names the reserved field, record
construction, ADT constructor
construction resolved through ordinary source module rules, including
constructor payloads made from nested supported mapping expressions, one pure
same-module converter function call, or one imported public pure converter
function call through a written `use` path or alias, and field selection from
an already supported structural mapping expression whose type has the selected
record field. An `Int` target field may also use `+`, `-`, `*`, and `/`
expressions whose operands are decoded schema-local `Int` fields, integer
literals, `Int`-returning converter calls, or nested supported integer
arithmetic mapping expressions. A `Bool` target field may use `==`, `!=`,
`<`, `<=`, `>`, and `>=` between those supported `Int` mapping operands, and
may compose those supported comparisons with `and`, `or`, and `not`. Converter
calls take one or more arguments. Each argument is either a schema-local field
reference or an
already implemented structural mapping expression made from schema-local
fields, records, ADT constructors, supported integer arithmetic mapping
expressions, pure converter calls, and nested combinations of those forms. The converter return
value is assigned to the
target field. A converter-call mapping assignment may name an explicit
same-module pure inverse converter or imported public pure inverse converter
through the same written import-path rules as the forward converter with
`inverse name` after the assignment expression. The inverse converter surface
is only a declared projection boundary for generated encode helpers;
converter names are not inferred from the forward function name.
Other ordinary calls, bare imported converter names, private imported
converters, non-`Int` converter arithmetic operands, effects, runtime
settings, stream state, and recovery behavior are not mapping expressions.
Mapping clauses are parsed, formatted, lowered, exposed to editor support,
and used by the generated decode slice described in
[execution.md](execution.md) when the schema has one structural mapping, or
multiple structural mappings selected by `when field == literal` or
`when field != literal`, ordered field-literal comparisons, or by boolean
selector expressions built from decoded schema-local `Int` fields, integer
literals, `==`, `!=`, `<`, `<=`, `>`, `>=`, `and`, `or`, and `not`, or by
direct selector calls to one pure same-module `Bool` converter function or one
imported public pure `Bool` converter function through a written `use` path or
alias, and all
assignment expressions use implemented decoded field types:
exact-width unsigned primitive fields as `Int`, `Flag8` fields as `Flag8`,
`Flag16be` fields as `Flag16be`, `Flag16le` fields as `Flag16le`,
`Flag24be` fields as `Flag24be`, `Flag24le` fields as `Flag24le`,
`Flag32be` fields as `Flag32be`, `Flag32le` fields as `Flag32le`,
`Flag40be` fields as `Flag40be`, `Flag40le` fields as `Flag40le`,
`Flag48be` fields as `Flag48be`, `Flag48le` fields as `Flag48le`,
`Flag56be` fields as `Flag56be`, `Flag56le` fields as `Flag56le`,
`Flag64be` fields as `Flag64be`, `Flag64le` fields as `Flag64le`,
length-bounded
`ByteView(length_field)`, `ByteView(left_length - right_length)`,
`ByteView(left_length + right_length)`, or
`ByteView(left_length * right_length)`, or
`ByteView(left_length / right_length)` payload fields as `ByteView`, bounded
`Repeat(count_field, Payload)` fields as lists of their payload value shape,
including `List<ByteView>` for `Repeat(count_field, ByteView(length_field))`,
closed nested dispatch payload fields as the nested schema record shape, and
closed mixed dispatch payload fields as the selected case payload shape within
the matching selector branch, closed recursive dispatch payload fields as the
selected mapping target record shape, and extension dispatch payload fields as
`SchemaDispatchPayload<T>`. Multiple selected mappings must all resolve to the
same decoded record shape. Selector comparisons may only compare a decoded
schema-local `Int` field with an integer literal using `==`, `!=`, `<`, `<=`,
`>`, or `>=`; direct converter selector calls must follow the same visibility,
purity, return-type, and argument rules as schema mapping converters and must
return `Bool`. Arbitrary ordinary calls, bare imported converter names, private
imported converters, record expressions as the selector root, schema-local
payload values, runtime settings, stream state, and unsupported arithmetic are
rejected as unsupported selectors. Selector clauses whose truth can be decided
from decoded `Int` field comparisons must not overlap for any concrete
assignment of their referenced `Int` fields.
Missing, duplicate, ambiguous, unknown-field, non-`Int`, and unsupported
selectors report
`schema.mapping_selection_required`, `schema.mapping_selection_ambiguous`,
`schema.mapping_selection`, or `schema.mapping_selection_unsupported`. The
predicate, primitive, dispatch, and mapping text are parsed and preserved as
source-surface syntax.
General schema decode, general schema encode beyond the exact-width
primitive, `Flag8`, `Flag16be`, `Flag16le`, `Flag24be`, `Flag24le`,
`Flag32be`, `Flag32le`, `Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`,
`Flag56be`, `Flag56le`, `Flag64be`, `Flag64le`, supported reserved-bit,
closed dispatch, extension dispatch, bounded repeated
primitive or nested schema field, length-bounded `ByteView`, and eligible
nested dispatch payload helper slices, general ADT constructor mapping beyond
schema-local structural expressions, support rather than rejection for
unbounded recursive dispatch payload schemas, dispatch payload schemas outside
the generated helper slice, arbitrary mapping expressions, and mapping
selection beyond this narrow decoded-field boolean and converter selector
slice are not implemented.
The checked diagnostics case
`../../examples/specification/check/schema-mapping-selection-diagnostics/`
pins the equality and inequality mapping selection boundary. The checked
diagnostics case
`../../examples/specification/check/schema-mapping-boolean-selector-diagnostics/`
pins boolean selector unsupported, unknown-field, non-`Int`, and overlap
diagnostics. The checked diagnostics case
`../../examples/specification/check/schema-mapping-converter-selector-diagnostics/`
pins converter selector return type, argument type, purity, visibility, and
written-import-path diagnostics. The checked runtime case
`../../examples/specification/run/binary-schema-nested-mapping-converter-selector-decode/`
pins nested converter calls inside direct converter selector arguments. The
checked diagnostics case
`../../examples/specification/check/schema-mapping-expression-boundary-diagnostics/`
pins unsupported mapping expression, unresolved constructor, constructor
arity, direct and nested constructor payload type, non-`Int` arithmetic operand,
and unsupported arithmetic expression diagnostics. The checked diagnostics case
`../../examples/specification/check/schema-mapping-bool-comparison-diagnostics/`
pins non-`Int` comparison operands, non-`Bool` comparison targets, and
unsupported comparison and boolean-composition operand shapes. The checked
runtime case
`../../examples/specification/run/binary-schema-mapping-bool-composition-decode/`
pins `Bool` mapping assignment composition with `and`, `or`, and `not`. The
checked diagnostics case
`../../examples/specification/check/schema-mapping-converter-diagnostics/`
pins unresolved converter, converter arity, converter input type, converter
return type through `schema.mapping_converter_return`, converter purity, and
unsupported converter argument expression diagnostics.
The checked diagnostics case
`../../examples/specification/check/schema-imported-mapping-converter-diagnostics/`
pins imported converter visibility and missing written import-path diagnostics.
The checked diagnostics case
`../../examples/specification/check/binary-schema-recursive-dispatch-payload-diagnostics/`
pins recursive dispatch payload schemas rejected outside the selected
same-module or public imported length-bounded dispatch boundary.
The checked acceptance case
`../../examples/specification/check/schema-two-argument-mapping-converters/`
pins same-module and imported public two-argument converter calls.
The checked diagnostics case
`../../examples/specification/check/schema-two-argument-mapping-converter-diagnostics/`
pins rejected second arguments while nested converter calls inside
two-argument converter calls remain supported.
The checked acceptance case
`../../examples/specification/check/schema-three-argument-mapping-converters/`
pins same-module and imported public three-argument converter calls.
The checked runtime cases
`../../examples/specification/run/binary-schema-four-argument-mapped-converter-decode/`
and
`../../examples/specification/run/binary-schema-imported-four-argument-mapped-converter-decode/`
pin same-module and imported public four-argument converter calls through
generated decode mapping.
The checked runtime cases
`../../examples/specification/run/binary-schema-five-argument-mapped-converter-decode/`
and
`../../examples/specification/run/binary-schema-imported-five-argument-mapped-converter-decode/`
pin same-module and imported public five-argument converter calls through
generated decode mapping.
The checked runtime cases
`../../examples/specification/run/binary-schema-mapped-converter-many-argument-decode/`
and
`../../examples/specification/run/binary-schema-imported-mapped-converter-many-argument-decode/`
pin same-module and imported public converter calls with more than five
supported structural arguments through generated decode mapping.
The checked diagnostics case
`../../examples/specification/check/schema-three-argument-mapping-converter-diagnostics/`
pins converter arity mismatches and rejected fifth arguments
while nested converter calls inside five-argument converter calls remain
supported.
Eligible binary schemas whose fields are visible exact-width unsigned
primitives, including standalone `UInt1` through `UInt7` fields that consume
one byte each and consecutive visible-only `UInt1` through `UInt7` groups of
at least two fields whose widths complete exactly one byte, one two-byte
big-endian storage unit, one three-byte big-endian storage unit, or one
four-byte big-endian storage unit, one five-byte big-endian storage unit, or
one six-byte big-endian storage unit,
`Flag8`,
`Flag16be`, `Flag16le`, `Flag24be`,
`Flag24le`,
`Flag32be`, `Flag32le`, `Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`,
`Flag56be`, `Flag56le`, `Flag64be`, and `Flag64le` bitset fields, supported
byte-aligned `ReservedBits(width, value)` fields,
the supported `ReservedBits(1, 0)` before `UInt31be` layout, the supported
`ReservedBits(2, 0)` and `ReservedBits(9, 0)` before `UInt8` byte-prefix
layouts, supported
packed prefix `ReservedBits(width, value)` plus `UIntN` layouts whose widths
sum to eight, sixteen, twenty-four, or thirty-two bits, including the
two-field `ReservedBits(15, value)` plus `UInt1` boundary, supported `UIntN`
plus reserved suffix layouts whose widths sum to eight, sixteen,
twenty-four, thirty-two, forty, forty-eight, fifty-six, or sixty-four bits,
supported visible `UInt8` plus non-byte-aligned multi-byte
`ReservedBits(width, value)` suffix layouts that fit in one three-byte
through eight-byte big-endian storage unit with low padding,
supported `UIntN` plus middle
`ReservedBits(width, value)` plus `UIntN` layouts whose widths sum to eight,
sixteen, twenty-four, or thirty-two bits,
including the narrow two-byte interleaved middle layout with a sub-byte
visible `UIntN`, a reserved field, `UInt8`, and a final sub-byte visible
`UIntN`,
supported `ReservedBits(width, value)` plus two visible sub-byte or
byte-width `UIntN` prefix groups whose widths sum to eight, sixteen,
twenty-four, thirty-two, forty, forty-eight, fifty-six, or sixty-four bits,
supported two-byte suffix groups where two visible `UIntN` fields, the second
one `UInt8`, precede a non-byte-aligned `ReservedBits(width, value)` field,
supported consecutive visible-only `UInt1` through `UInt7` groups whose widths
sum to eight, sixteen, twenty-four, thirty-two, forty, or forty-eight bits,
supported
consecutive non-byte-aligned `UIntN` and
`ReservedBits(width, value)` groups whose widths sum to eight, sixteen,
twenty-four, thirty-two, forty, forty-eight, fifty-six, or sixty-four bits,
bounded `Repeat(count_field, Payload)` fields whose count names an earlier
visible exact-width unsigned `Int` field, bounded
`Repeat(left_count - right_count, Payload)` and
`Repeat(left_count + right_count, Payload)` and
`Repeat(left_count * right_count, Payload)` and
`Repeat(left_count / right_count, Payload)` fields whose operands both name
earlier visible exact-width unsigned `Int` fields, and whose payload is either
`UInt8`, `UInt16be`, `UInt16le`, `UInt24be`, `UInt24le`, `UInt31be`,
`UInt31le`, `UInt32be`, `UInt32le`, `UInt40be`, `UInt40le`, `UInt48be`,
`UInt48le`, `UInt56be`, `UInt56le`, `UInt64be`, `UInt64le`, or an eligible
nested binary schema payload, or
`ByteView(length_field)` whose length names an earlier visible exact-width
unsigned `Int` field,
length-bounded `ByteView(length_field)` payload fields whose length names an
earlier visible exact-width unsigned `Int` field,
`ByteView(left_length - right_length)` payload fields whose operands both name
earlier visible exact-width unsigned `Int` fields,
`ByteView(left_length + right_length)` payload fields whose operands both name
earlier visible exact-width unsigned `Int` fields, or
`ByteView(left_length * right_length)` payload fields whose operands both name
earlier visible exact-width unsigned `Int` fields, or
`ByteView(left_length / right_length)` payload fields whose operands both name
earlier visible exact-width unsigned `Int` fields,
closed `Dispatch(tag_field, tag => Payload, ...)` fields, and
extension-tolerant `ExtensionDispatch(tag_field, length_field, tag => Payload,
...)` fields whose tag and length names are earlier visible exact-width fields
and whose cases are exact-width unsigned primitive payloads or eligible
nested binary schema payloads, also expose generated
`byte_encode_<schema>` helpers described in [execution.md](execution.md);
one structural `map to Target` clause can make that helper accept the mapping
target record shape when every visible encode field, including `Flag8`,
`Flag16be`, `Flag16le`, `Flag24be`, `Flag24le`, `Flag32be`, `Flag32le`,
`Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`, `Flag56be`, `Flag56le`,
`Flag64be`, and `Flag64le` fields, is assigned from a projectable
schema-local field reference. Multiple selected
`map to Target when field <literal-comparison> literal` clauses using `==`,
`!=`, `<`, `<=`, `>`, or `>=` can make the helper accept that same target
record shape when all selected mappings resolve to it and every schema-local
encode field, including the selector field, projects back from the selected
target record through direct source-field assignments.
For closed dispatch fields whose cases mix primitive and nested schema payload
decoded shapes, selected equality mappings on the dispatch tag field can make
the helper accept the shared target record when the mappings cover every case
exactly once and each branch projects the case-local payload shape back to the
dispatch payload field.
`ReservedBits(width, value)` field layouts outside those helper slices are
rejected before typed IR is emitted; checked JSON and human diagnostics are in
`../../examples/specification/check/schema-reserved-bit-layout-diagnostics/`
and `../../examples/specification/check/schema-reserved-bit-layout-human/`.
Projectable expressions for the one-clause form are direct schema-local field
references, record expressions whose fields are direct schema-local visible
field references, field selection from such a record expression when the
selected field maps directly to one schema-local visible field, or a direct
ADT constructor call whose payload arguments use those projectable field and
record-expression forms already supported by the generated encode helper.
Constructor payload arguments may nest ADT constructor calls when their
leaves stay within those same projectable forms.
Single-payload constructor wrappers remain limited to the existing
single-constructor flag and exact-width integer cases unless the payload is
that record-expression slice or a supported nested constructor projection.
Selected mappings that cannot reconstruct every schema-local encode field
through direct source-field assignments, mapping
expressions that cannot be projected back to schema-local fields, recursive
dispatch payload schemas, dispatch payload schemas outside the generated
helper slice, non-byte-aligned reserved fields outside the supported packed,
middle, suffix-group, and `UInt31be` shared-bit
layouts, and derived codec encode execution over unsupported schemas are
outside that encode helper slice.
Schema declarations do not create ordinary value bindings or ordinary type
declarations.

Top-level `codec Name for SchemaName ...` and
`pub codec Name for SchemaName ...` declarations are implemented as source
module items. A codec head lists one or both explicit directions, `decode` and
`encode`. The body contains one implementation clause for each listed
direction: `derive decode`, `derive encode`, `decode with function_name`, or
`encode with function_name`. The parser reports declaration-shape errors for
empty, unknown, or duplicate directions, missing clauses, clauses for unlisted
directions, and duplicate clauses. The source model preserves codec visibility,
schema ownership, directions, and body clauses for metadata, formatting,
editor support, and checker boundaries.

A codec schema reference resolves through schema-aware name lookup. Bare
`codec Name for SchemaName` references are limited to schemas and schema
aliases declared in the codec's own module. Qualified
`codec Name for imported::SchemaName` references require a matching written
`use imported` path or alias in the codec's module, and the target schema or
schema alias must be `pub`. A `pub schema Public = imported::Packet` member
alias publishes an existing public schema through the declaring module's
public path without creating a wrapper schema, schema identity, generated
codec alias, ordinary type binding, generated helper binding, or schema-local
field binding. Schema alias targets use schema-aware lookup. Missing, private,
function, source ADT type, and codec targets are rejected at the alias
declaration. Imported private schemas report `name.visibility` at the codec
declaration. Missing schema targets report `name.unresolved`; ordinary
functions, source ADT types, and codec items at the referenced path report
`name.kind_mismatch` instead of being treated as schemas. Importing or
referencing a schema does not import schema-local field names or create
ordinary type bindings. Executable decode codec item calls are provided by
valid hand-written decode implementations and by `derive decode` for the
eligible generated binary schema decode-step slice, not by schema references
themselves.

A `decode with function_name` clause must resolve to an ordinary function in
the codec's module with exactly `ByteView` and `ByteOffset` parameters and a
`DecodeStep<T>` return type. Invalid decode signatures report
`codec.decode_signature` at the codec implementation clause, with related
context pointing to the referenced function when it is available. When the
referenced schema has an implemented structural mapping slice, the `T` value
type must match the mapping target record shape, including selected mappings
that all resolve to that same record shape; mismatches report
`codec.decode_value_type` at the codec implementation clause. A codec with a
hand-written `decode with` clause is callable through the codec item name in
its declaring module, or through a written import-qualified module path when
the codec is `pub`. That call takes the same `ByteView` and `ByteOffset`
arguments as the referenced function. It returns valid `Decoded`,
`NeedMore`, and `Invalid` results unchanged, and projects an oversized
consumed count to `codec.consumed_count_invalid` as specified in
`execution.md`.
`derive decode` codecs are callable through the same visibility and import
rules when their schema is eligible for `byte_decode_step_<schema>`, and the
call returns that generated helper's `DecodeStep<T>` result. For the
implemented structural mapping slice, `T` is the mapping target record
shape when each assignment source has the same implemented decoded field type
as the target field and all selected mappings resolve to that same record
shape. Unsupported generated decode helper eligibility reports
`codec.derive_helper_unsupported` at the `derive decode` clause. Bare imported
codec names are not ordinary call targets.

An `encode with function_name` clause must resolve to an ordinary function in
the codec's module with an `EncodeStep<TState>` return type. When the
referenced schema has an implemented structural mapping slice, the function's first
parameter must match the mapping target record shape. Invalid encode
signatures report `codec.encode_signature`; mapped value parameter mismatches
report `codec.encode_value_type` at the codec implementation clause, with
related context pointing to the referenced function when it is available.
When the clause is valid, the codec item name is an ordinary call target in
the declaring module, or through a written import-qualified module path when
the codec is `pub`. The call uses the referenced function's parameters and
returns its `EncodeStep<TState>` value unchanged. General codec-generated
decode functions are not implemented. Generated
`byte_decode_<schema>` helpers for the eligible binary schema slice, generated
`byte_encode_<schema>` helpers for the exact-width including standalone
`UInt1` through `UInt7`, opt-in visible flag bitset fields, supported
reserved-bit, closed dispatch, extension dispatch, length-bounded `ByteView`,
repeated primitive, nested schema, and `ByteView(length_field)` payloads,
quotient-count repeat fields, same-module or imported public nested dispatch
payload encode slices, same-module nested dispatch
`ByteView(left_length + right_length)` payload helper slices, and same-module
recursive closed and extension dispatch payload slices,
their `byte_decode_step_<schema>` incremental decode-step counterparts,
derived decode codec calls over that decode-step slice, and derived encode
codec calls over that encode helper slice, including the combined non-HTTP
general helper shape checked by
`../../examples/specification/run/derived-codec-general-helper-boundary/`, are
covered by [execution.md](execution.md).
When a mapped schema cannot expose the mapping target through a generated
encode boundary, the `derive encode` clause reports
`codec.derive_helper_unsupported`.
The implemented direct structural mapping slice exposes that target record as
the generated encode boundary.

Documentation comments may reference schemas with `{@schema Name}` or
`{@schema module::Name}`. These references use schema-aware lookup, not value
or type lookup. Bare references resolve schemas and schema aliases in the same
module. Qualified references require a matching written `use` path, including
nested module paths such as `use app::nested`, and a public schema or public
schema alias. Missing, private, function, source ADT type, and codec targets
are rejected at the documentation reference span. Schema references in
documentation do not expose schema-local field names, generated helper names,
codec names, or ordinary source type bindings.

Executable specification fixture metadata reuses the same schema-aware lookup
rules for optional binary fixture `schema` references. Fixture references do
not create source bindings or expose generated helpers.

## Expressions

See [source-surface-full.md#expressions](source-surface-full.md#expressions).

## Contract Predicates

See
[source-surface-full.md#contract-predicates](source-surface-full.md#contract-predicates).
