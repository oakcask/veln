# Source Surface

This file specifies the source subset implemented by the parser and AST.

## Grammar

The formal grammar block is generated from the executable Prolog source
surface specification. Keep semantic behavior, diagnostics, and rationale in
the surrounding prose.

<!-- source-surface-grammar:start -->
```text
Module        ::= UseDecl* Item*
UseDecl       ::= "use" ModulePath ImportSource? NL
ImportSource  ::= "from" PackageString
ModulePath    ::= Name ("::" Name)*
PackageString ::= String
IntLiteral    ::= ASCII decimal digit+
Item          ::= Function | TestDecl | TypeDecl | SchemaDecl | PublicAlias
                  | CodecDecl
Function      ::= "pub"? "fn" Name "(" ParamList? ")" Return? Effects? NL
                  Contract* Body "end" NL?
TestDecl      ::= "test" Name "(" ")" Return Effects? NL
                  Contract* Body "end" NL?
TypeDecl      ::= "pub"? "type" Name TypeParamList? NL TypeVariant+ "end" NL?
SchemaDecl    ::= "pub"? "schema" Name NL SchemaFormat NL SchemaField+ SchemaValidation? SchemaMapping* "end" NL?
SchemaFormat  ::= "format" "binary" NL
SchemaField   ::= Name ":" SchemaFieldType SchemaFieldWhere? NL
SchemaFieldType ::= TypeText | ReservedBitsPrimitive | RepeatPrimitive
ReservedBitsPrimitive ::= "ReservedBits" "(" IntLiteral "," IntLiteral ")"
RepeatPrimitive ::= "Repeat" "(" CountExpr "," TypeText ")"
CountExpr ::= Name | Name ("-" | "+") Name
SchemaFieldWhere ::= "where" ContractPredicate
SchemaValidation ::= "validate" ContractPredicate NL
SchemaMapping ::= "map" "to" MemberPath SchemaMappingSelector? NL SchemaMappingAssignment+
SchemaMappingSelector ::= "when" Expr
SchemaMappingAssignment ::= Name "=" Expr NL
CodecDecl     ::= "pub"? "codec" Name "for" MemberPath CodecDirections NL
                  CodecImplementation* "end" NL?
CodecDirections ::= CodecDirection+
CodecDirection ::= "decode" | "encode"
CodecImplementation ::= "derive" CodecDirection NL
                  | CodecDirection "with" Name NL
PublicAlias   ::= "pub" ("fn" | "type" | "schema") Name "=" MemberPath NL
TypeParamList ::= "<" Name ("," Name)* ","? ">"
TypeVariant   ::= "pub"? UpperName TypeVariantFields? NL
TypeVariantFields ::= "(" TypeVariantField ("," TypeVariantField)* ","? ")"
                  | "{" TypeVariantField ("," TypeVariantField)* ","? "}"
TypeVariantField ::= Name ":" TypeText | TypeText
ParamList     ::= Param ("," Param)* ","?
Param         ::= Name (":" VariadicMarker? TypeText)?
VariadicMarker ::= "..."
Return        ::= "->" ResultBinding? TypeText
ResultBinding ::= Name ":"
Effects       ::= "effects" "[" EffectList? "]"
EffectList    ::= Name ("," Name)* ","?
Contract      ::= ("require" | "ensure" | "invariant") ContractPredicate NL
Body          ::= (LetLine | ExprLine)*
LetLine       ::= "let" LetPattern (":" TypeText)? "=" Expr NL
LetPattern    ::= "_" | BindingName | RecordPattern
ExprLine      ::= Expr NL
Expr          ::= PrefixExpr (BinaryOp PrefixExpr)*
PrefixExpr    ::= ("not" | "-") PrefixExpr | PostfixExpr
PostfixExpr   ::= PrimaryExpr (Call | TypeArgs | FieldAccess | "?")*
PrimaryExpr   ::= Hole | Literal | NamePath | "(" Expr ")" | "()"
                  | Record | Dict | List | Match | If
Call          ::= "(" ArgList? ")"
ArgList       ::= Expr ("," Expr)* ","?
TypeArgs      ::= "<" TypeText ("," TypeText)* ","? ">"
FieldAccess   ::= "." Name
Record        ::= "{" (Name ":" Expr) ("," Name ":" Expr)* ","? "}"
Dict          ::= "{" Expr ":" Expr ("," Expr ":" Expr)* ","? "}"
List          ::= "[" ArgList? "]"
Match         ::= "match" Expr NL MatchArm+ "end"
MatchArm      ::= Pattern "=>" Expr NL
If            ::= "if" Expr NL Expr NL ElseIf* "else" NL Expr NL "end"
ElseIf        ::= "else" "if" Expr NL Expr NL
Pattern       ::= "_" | BindingName | Literal | ConstructorPattern | RecordPattern
ConstructorPattern ::= ConstructorName "(" PatternList? ")" | ConstructorName
ConstructorName ::= UpperName | Name "::" Name ("::" Name)*
RecordPattern ::= "{" PatternFieldList? "}"
PatternList   ::= Pattern ("," Pattern)* ","?
PatternFieldList ::= PatternField ("," PatternField)* ","?
PatternField  ::= Name ":" Pattern
MemberPath    ::= Name ("::" Name)*
```
<!-- source-surface-grammar:end -->

`Name` is an identifier. `UpperName` is an identifier whose first character is
uppercase. `BindingName` is an unqualified identifier whose first character is
not uppercase. `TypeText` is collected from source and parsed by the semantic
type parser. Contract predicates parse through a narrower predicate production
before semantic contract validation.

A function declaration parameter may be variadic by writing `name: ...T`.
Only the final parameter may be variadic, and a function may have at most one
variadic parameter. The marker belongs to parameter syntax; ordinary local,
field, record, and type argument annotations reject `...T`.

Schema declarations are top-level source module items. `schema Name` is
private to its source module, and `pub schema Name` records public schema
ownership for the declaring module. The implemented schema body slice requires
one `format binary` clause before any schema fields, followed by one or more
`name: TypeText` field lines. A field line may end with a field-local `where`
predicate after the type text, such as `padding_length: UInt8 where
padding_length <= length`. Binary schema fields also accept exact-width
unsigned primitive names `UInt1` through `UInt8`, `UInt16be`, `UInt16le`,
`UInt24be`, `UInt24le`, `UInt31be`, `UInt31le`, `UInt32be`, `UInt32le`,
`UInt40be`, `UInt40le`, `UInt48be`, `UInt48le`, `UInt56be`, `UInt56le`,
`UInt64be`, and `UInt64le`; those names are schema-local representation
vocabulary, not ordinary source types or values. Binary schema fields also
accept `Flag8`, `Flag16be`, `Flag16le`, `Flag24be`, `Flag24le`, `Flag32be`,
`Flag32le`, `Flag40be`, `Flag40le`, `Flag48be`, `Flag48le`, `Flag56be`,
`Flag56le`, `Flag64be`, and `Flag64le` as visible flag bitset fields that
decode and encode through source-visible flag values instead of raw `Int`
fields.
Binary schema fields also accept the
`ReservedBits(width, value)` primitive
spelling when `width` and `value` are literal non-negative integers, such as
`ReservedBits(1, 0)`, a byte-aligned reserved field, or a supported packed
reserved prefix or suffix paired with a visible unsigned field, and supported
reserved prefix groups followed by two visible sub-byte or byte-width
unsigned fields in one-byte, two-byte, three-byte, four-byte, five-byte,
six-byte, seven-byte, or eight-byte big-endian storage, or a supported
two-byte suffix group where two visible unsigned fields, one byte-width, are
followed by a non-byte-aligned reserved field.
Binary schema
fields
also accept `Repeat(count_field, Payload)` when `count_field` names a
previously decoded visible `Int` field in the same schema, and
`Repeat(left_count - right_count, Payload)`,
`Repeat(left_count + right_count, Payload)`, and
`Repeat(left_count * right_count, Payload)`, and
`Repeat(left_count / right_count, Payload)` when both operands name earlier
visible `Int` fields in the same schema. `Payload` is one of the implemented
byte-aligned exact-width unsigned primitives or an eligible nested binary
schema payload. `Repeat(count_field, ByteView(length_field))` is accepted when
both references name earlier visible `Int` fields in the same schema.
Length-bounded `ByteView(length_field)`,
`ByteView(left_length - right_length)`,
`ByteView(left_length + right_length)`, and
`ByteView(left_length * right_length)`, and
`ByteView(left_length / right_length)` payload fields are accepted when every
length operand names an earlier visible `Int` field in the same binary schema.
A repeated primitive field decodes and encodes as `List<Int>`; a repeated
nested schema field decodes and encodes as a list of the nested schema's
decoded record shape; and a repeated `ByteView(length_field)` field decodes
and encodes as `List<ByteView>`. Missing, forward, or non-`Int` repeat count
references report `schema.repeat_reference`; missing, forward, or non-`Int`
byte-view length references report `schema.byte_view_reference`. Binary schema
fields also accept the closed dispatch types
`Dispatch(tag_field, tag => Payload, ...)` and
`Dispatch(tag_field, length_field, tag => Payload, ...)`, and the
extension-tolerant type
`ExtensionDispatch(tag_field, length_field, tag => Payload, ...)`, when the
referenced fields were decoded earlier in the same schema and case payloads
are implemented exact-width unsigned primitives or eligible nested binary
schema payloads. Nested payload schema names must resolve to earlier
same-module binary schema items or public imported binary schemas named
through written `use` paths, and the named schemas must themselves be
eligible for the generated binary schema helper path, including
length-bounded `ByteView(length_field)` fields whose length names an earlier
visible `Int` field in that nested schema. The tag and length
references must resolve to visible `Int` fields. Same-module recursive closed
or extension-tolerant dispatch payload schemas, same-module dispatch cases
that name a separate eligible recursive payload schema, and public imported
recursive payload schemas named through written `use` paths, are accepted
only in the length-bounded forms when selected mappings cover every known case
and resolve to one record shape, with at least one non-recursive case as the
base case. A length-bounded parent dispatch field without selected mappings
may also name an earlier same-module recursive payload schema or a public
imported recursive payload schema through a written `use` path when the
payload schema has that same bounded recursive helper support, the parent has
at least one non-recursive primitive case, and the parent requires only
generated decode helper support. The closed parent payload type is the
payload schema's recursive mapped payload type; the extension-tolerant parent
wraps that payload type in `SchemaDispatchPayload`.
Exact-width primitive names used outside `format binary` schema field type
positions report `schema.exact_width_primitive`. Missing
`ReservedBits` arguments or non-literal arguments report
`schema.reserved_bits_primitive`. Missing, forward, or non-`Int` tag and
length references report `schema.dispatch_reference`. Nested payload names
that are missing, non-schema, private imported, non-binary, forward,
unbounded recursive, missing the decode helper required by parent decode,
field-reference-ineligible `ByteView` payload layout, or incompatible report
`schema.dispatch_payload`. Parent decode helpers, generated decode-step
helpers, and `derive decode` require only the nested schema's decode helper;
generated encode helpers and `derive encode` also require the nested schema's
encode helper.
Helper-slice payload diagnostics name the expected generated decode and
encode helpers in structured fields and keep the payload schema declaration
in related notes. When the nested schema has a field layout that prevents
helper exposure, the diagnostic details and related notes also name the nested
schema field path, the unavailable helper directions, and the specific
unsupported `ReservedBits`, `ByteView`, or mapping-projection layout fact.
The checked field-reference diagnostics case is
`../../examples/specification/check/binary-schema-field-reference-diagnostics/`;
the checked dispatch payload diagnostics case is
`../../examples/specification/check/binary-schema-dispatch-payload-diagnostics/`;
the checked imported recursive payload acceptance case is
`../../examples/specification/check/binary-schema-imported-recursive-dispatch-payload-accepted/`;
the checked helper-eligibility detail cases are
`../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-diagnostics/`
and
`../../examples/specification/check/binary-schema-dispatch-payload-helper-eligibility-human/`.

A schema may end with one or more structural mapping clauses:

```text
map to FrameHeader
  length = length
  kind = kind
  stream_id = stream_id
```

The mapping target is a member path naming an ordinary source value shape. Each
assignment line must explicitly name a target field on the left and a
schema mapping expression on the right. The implemented expression slice
supports schema-local field references, record construction, ADT constructor
construction resolved through ordinary source module rules, one pure
same-module converter call, and one imported public pure converter call
through a written `use` path or alias, and field selection from an already
supported structural mapping expression whose type has the selected record
field. An `Int` target field may also use `+`, `-`, `*`, and `/` expressions
whose operands are decoded schema-local `Int` fields, integer literals,
`Int`-returning converter calls, or nested supported integer arithmetic
mapping expressions. A `Bool` target field may use `==`, `!=`, `<`, `<=`,
`>`, and `>=` between supported `Int` mapping operands, and may compose those
supported comparisons with `and`, `or`, and `not`. Converter calls take one,
two, three, four, or five arguments. Each argument is either a schema-local field
reference or an already implemented structural mapping expression made from
schema-local fields, records, ADT constructors, supported integer arithmetic
mapping expressions, pure converter calls, and nested combinations of those
forms.
Duplicate left-hand targets, missing left-hand targets, and bare schema-field
lines are parse diagnostics; reserved bits and other representation fields are
omitted unless explicitly assigned. The parser, formatter, lowered AST, and
editor token collector preserve mapping clauses as source metadata. The
generated binary decode helper uses one eligible structural mapping clause, or
multiple eligible mapping clauses selected by `when field == literal`,
`when field != literal`, ordered field-literal comparisons, or by narrow
boolean selector expressions over decoded schema-local `Int` fields, or by a
direct selector call to one pure same-module `Bool` converter or imported
public pure `Bool` converter named through a written `use` path or alias, when
all schema fields are implemented exact-width unsigned primitives, supported
reserved-bit fields, bounded repeated primitive or nested schema fields, closed dispatch
fields, or extension dispatch fields and the target resolves to matching
record fields. Multiple selected mappings must decode to the same record
shape. Selector comparisons may only compare decoded schema-local `Int` fields
with integer literals, and selector clauses must not overlap for any concrete
assignment of referenced selector fields. Converter selector calls follow the
same visibility, purity, return-type, and argument rules as schema mapping
converters and must return `Bool`. Missing selectors report
`schema.mapping_selection_required`, duplicate or overlapping selectors report
`schema.mapping_selection_ambiguous`, and unsupported selector
or target-shape boundaries report `schema.mapping_selection` or
`schema.mapping_selection_unsupported`. Target-field resolution outside that
record slice, arbitrary calls, non-`Int` converter arithmetic operands,
value-dependent mapping beyond decoded-field and converter-call integer
arithmetic and supported `Int` comparisons composed for `Bool` target fields,
and encode-side mapping outside the selected structural slice are not
implemented.
The executable diagnostics case
`../../examples/specification/check/schema-mapping-selection-diagnostics/`
keeps the mapping-selection boundary executable.
The executable diagnostics case
`../../examples/specification/check/schema-mapping-expression-boundary-diagnostics/`
keeps unsupported mapping expression, unresolved constructor, constructor
arity, direct and nested constructor payload type, non-`Int` arithmetic
operand, and unsupported arithmetic expression diagnostics executable.
The executable diagnostics cases
`../../examples/specification/check/schema-mapping-converter-arithmetic-diagnostics/`
and
`../../examples/specification/check/schema-mapping-converter-arithmetic-diagnostics-human/`
keep converter arithmetic operand resolution, arity, input, return, purity,
and unsupported argument diagnostics executable.
The executable diagnostics case
`../../examples/specification/check/schema-mapping-converter-diagnostics/`
keeps unresolved converter, converter arity, converter input type, converter
return type, converter purity, and unsupported converter argument expression
diagnostics executable.
The executable diagnostics case
`../../examples/specification/check/schema-imported-mapping-converter-diagnostics/`
keeps imported converter visibility and missing written import-path
diagnostics executable.
The executable diagnostics case
`../../examples/specification/check/schema-mapping-converter-selector-diagnostics/`
keeps converter selector return type, argument type, purity, visibility, and
written import-path diagnostics executable.
`../../examples/specification/run/binary-schema-nested-mapping-converter-selector-decode/`
keeps nested converter calls inside direct converter selector arguments
executable.
`../../examples/specification/check/schema-two-argument-mapping-converters/`
keeps same-module and imported public two-argument converter calls executable.
The executable diagnostics case
`../../examples/specification/check/schema-two-argument-mapping-converter-diagnostics/`
keeps rejected second arguments executable while nested converter calls inside
two-argument converter calls remain accepted.
`../../examples/specification/check/schema-three-argument-mapping-converters/`
keeps same-module and imported public three-argument converter calls
executable.
The executable runtime cases
`../../examples/specification/run/binary-schema-four-argument-mapped-converter-decode/`
and
`../../examples/specification/run/binary-schema-imported-four-argument-mapped-converter-decode/`
keep same-module and imported public four-argument converter calls executable
through generated decode mapping.
The executable runtime cases
`../../examples/specification/run/binary-schema-five-argument-mapped-converter-decode/`
and
`../../examples/specification/run/binary-schema-imported-five-argument-mapped-converter-decode/`
keep same-module and imported public five-argument converter calls executable
through generated decode mapping.
The executable diagnostics case
`../../examples/specification/check/schema-three-argument-mapping-converter-diagnostics/`
keeps unsupported six-argument converter calls and rejected fifth arguments
executable while nested converter calls inside five-argument converter calls
remain accepted.

The parser preserves the predicate, primitive, and mapping text with the owning
schema for diagnostics and editor support. Eligible binary schemas whose
fields are visible exact-width unsigned primitives, including standalone
`UInt1` through `UInt7` fields that consume one byte each and consecutive
visible-only `UInt1` through `UInt7` groups of at least two fields whose
widths complete exactly one byte, one two-byte big-endian storage unit, or
one three-byte big-endian storage unit, or one four-byte big-endian storage
unit,
plus the supported
byte-aligned `ReservedBits(width, value)` fields, the supported
`ReservedBits(1, 0)` before `UInt31be` layout, the supported
`ReservedBits(2, 0)` and `ReservedBits(9, 0)` before `UInt8` byte-prefix
layouts, supported
prefix `ReservedBits(width, value)` plus `UIntN` layouts whose widths sum to
eight, sixteen, twenty-four, or thirty-two bits, supported `UIntN` plus
reserved suffix layouts whose widths sum to eight, sixteen, twenty-four,
thirty-two, forty, forty-eight, fifty-six, or sixty-four bits, supported
visible `UInt8` plus non-byte-aligned multi-byte
`ReservedBits(width, value)` suffix layouts that fit in one three-byte through
eight-byte big-endian storage unit with low padding, supported `UIntN` plus
middle
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
supported consecutive non-byte-aligned `UIntN` and
`ReservedBits(width, value)` groups whose widths sum to eight, sixteen,
twenty-four, thirty-two, forty, forty-eight, fifty-six, or sixty-four bits,
bounded repeat fields whose count is an earlier visible exact-width field or
the difference, sum, product, or integer quotient of two earlier visible
exact-width fields,
including
`Repeat(count_field, ByteView(length_field))` when the length field is also
earlier visible,
length-bounded `ByteView(length_field)`
fields whose length names an earlier visible exact-width field,
`ByteView(left_length - right_length)` fields whose operands both name earlier
visible exact-width fields,
`ByteView(left_length + right_length)` fields whose operands both name earlier
visible exact-width fields, and
`ByteView(left_length * right_length)` fields whose operands both name earlier
visible exact-width fields, and
`ByteView(left_length / right_length)` fields whose operands both name earlier
visible exact-width fields, closed
dispatch fields, and
extension-tolerant dispatch fields whose tag and length names are earlier
visible exact-width fields and whose cases are exact-width unsigned primitive
payloads or eligible nested binary schema payloads, expose generated
`byte_encode_<schema>` helpers routed from `execution.md`.
`ReservedBits(width, value)` field layouts outside those helper slices are
rejected before typed IR is emitted; checked JSON and human diagnostics are in
`../../examples/specification/check/schema-reserved-bit-layout-diagnostics/`
and `../../examples/specification/check/schema-reserved-bit-layout-human/`.
General schema
encode execution beyond those helper slices and schema decode outside the
narrow generated binary helper slices are not implemented. The narrow
primitive, field-local validation, mapped-record decode, dispatch decode, and
primitive, reserved-bit, length-bounded `ByteView`, closed dispatch,
extension dispatch, eligible nested dispatch payload encode, and same-module
recursive dispatch payload slices are routed from
`execution.md`. Field
names must be ordinary identifiers; names
beginning with `_` remain hole tokens and are rejected as schema field names.
Schema declarations do not create ordinary value bindings, ordinary source ADT
types, constructors, or general executable decode or encode functions.

Codec declarations are top-level source module items. `codec Name for
SchemaName decode`, `codec Name for imported::SchemaName encode`, and
`codec Name for SchemaName decode encode` are accepted, with optional leading
`pub` for public module ownership. The direction list must be non-empty and
cannot repeat `decode` or `encode`; other direction words are rejected. A codec
body contains one implementation clause for each listed direction:
`derive decode`, `derive encode`, `decode with function_name`, or
`encode with function_name`. A missing clause, a clause for an unlisted
direction, or a duplicate clause is a parse diagnostic. The parser and AST
preserve the codec name, referenced schema path, directions, visibility, and
body clauses for formatting, editor support, source metadata, and checker
boundaries.

The checker validates the implemented decode function boundary for
`decode with function_name`. The name must resolve to an ordinary function in
the same module as the codec declaration. The referenced function must take
exactly two parameters, first `ByteView` for the bounded input view and then
`ByteOffset` for the absolute base byte offset. Its return type must be
`DecodeStep<T>` for one source-visible decoded value type `T`. When the
referenced schema has an implemented structural `map to Target` slice, `T`
must match the mapped target record shape, including selected mappings that
all resolve to that same record shape. A missing function reports
`name.unresolved` at the `decode with` clause. A wrong parameter count,
parameter type, or return type reports `codec.decode_signature` at that clause
and includes related context pointing to the referenced function declaration.
A mapped value type mismatch reports `codec.decode_value_type` at that clause.
When the clause is valid, the codec item name is an ordinary call target for
the hand-written decode boundary in the declaring module. A `pub codec` can
also be called through a written import-qualified module path. The call
expects the same `ByteView` and `ByteOffset` arguments as the referenced
function. It returns valid `Decoded`, `NeedMore`, and `Invalid` results
unchanged, and projects an oversized consumed count to
`codec.consumed_count_invalid` as specified in `execution.md`. Importing the
codec item does not make the referenced decoder function or the codec schema
available through the importing module.

When a codec has `derive decode` and the referenced schema is eligible for the
generated `byte_decode_step_<schema>` helper, the codec item name is also an
ordinary decode call target in the declaring module. A `pub codec` can also be
called through a written import-qualified module path. The call expects
`ByteView` and `ByteOffset` arguments and returns that generated helper's
`DecodeStep<T>` result. For the implemented structural mapping slice, `T` is
the mapping target record shape when each assignment source has the
same implemented decoded field type as the target field and all selected
mappings resolve to that same record shape. Unsupported generated decode
helper eligibility reports `codec.derive_helper_unsupported` at the
`derive decode` clause. Bare imported codec names are not call targets, and
encode-only codecs do not expose this decode boundary.

The checker also validates the implemented encode function boundary for
`encode with function_name`. The name must resolve to an ordinary function in
the same module as the codec declaration. When the referenced schema has an
implemented structural `map to Target` slice, the referenced function's first
parameter must match the mapped target record shape. The referenced function
must return `EncodeStep<TState>` for one source-visible encoder state type
`TState`. A missing function reports `name.unresolved` at the `encode with`
clause. A wrong return type reports `codec.encode_signature`; a missing or
wrong mapped value parameter reports `codec.encode_value_type` at that clause.
Both diagnostics include related context pointing to the referenced function
declaration. When the clause is valid, the codec item name is an ordinary call
target for the hand-written encode boundary in the declaring module. A
`pub codec` can also be called through a written import-qualified module path.
The call expects the same parameters as the referenced function and returns
that function's `EncodeStep<TState>` value unchanged. Importing the codec item
does not make the referenced encoder function or the codec schema available
through the importing module.

When a codec has `derive encode`, a referenced schema whose implemented
structural mapping changes the value boundary uses the mapping target value
type when the generated encode helper can project that target record back to
schema-local fields through projectable field, record-expression, field
selection, or direct ADT constructor mapping expressions. Multiple selected
`map to Target when field <literal-comparison> literal` clauses using `==`,
`!=`, `<`, `<=`, `>`, or `>=` use that same target value type when all
selected mappings resolve to it and every schema-local encode field,
including the selector field, projects back from the selected target record
through direct source-field assignments. Other mapped encode boundaries are
rejected at the clause with `codec.derive_helper_unsupported`. This keeps
mapped schemas from silently exposing the schema-local generated encode record
as the codec value boundary.

Codec declarations do not generate general executable decode or encode
functions beyond the derived decode and derived encode slices routed from
`execution.md`.

In expression position, `{}` and brace literals whose first entry is a bare
`name: value` field parse as records. Other brace literals with `key: value`
entries parse as dictionaries, including keys that are identifier-led
expressions such as `seed + 1`.

Function and test declarations can contain multiple body lines between their
header and closing `end`. Expression newlines end the current body line except
inside grouping forms. Parentheses, brackets, braces, and `match` expressions
keep their inner newlines within the same expression; indentation is formatting
only and does not define parse structure. If a complete body expression is
followed by another token before the line ends, the parser reports
`parse.expected_newline` at that token. If a `let` pattern leaves extra pattern
tokens before the `=`, the parser reports `parse.pattern` at the first extra
token.

Formatter indentation and canonical comment spelling are command behavior. See
[commands-full.md#veln-fmt-path](commands-full.md#veln-fmt-path) for the canonical
`veln fmt` layout.

`#` starts an ordinary line comment and runs through the end of the line. `##`
starts a documentation line comment. `//` is not an ordinary line comment
marker, and `///` is not a documentation line comment marker.

The final expression line is the returned value. If a body has no final
expression line, the omitted tail expression returns `()`. A non-`()`
declared return type reports `type.mismatch` with
`actual_type_source: "implicit_unit"`.

When a declaration returns a function type that itself carries effects, the
function-type effect list belongs to the returned value:
`-> fn(String) -> () effects [stdio]`. An additional declaration-level
`effects [...]` clause after that return type belongs to the enclosing
declaration and must be non-empty when written.

Omitting a declaration-level `effects [...]` clause means the declared effect
set is empty. A function or test declaration may write a declaration-level
effect clause only when the list contains at least one effect label. The parser
keeps `effects []` in the AST so semantic checking can report
`effect.empty_declaration` with repair notes, but the spelling is not accepted
as valid source behavior. Function type annotations may still use
`effects []` to describe a pure callable value.

`let _ = expr` evaluates the expression and discards the resulting value. It
does not introduce a local binding, and later expressions cannot reference the
discard target. A type annotation on the wildcard target still checks the
right-hand expression against that type. `let` also accepts binding and record
patterns. A record let pattern binds nested field values from the right-hand
record expression. Constructor let patterns bind payload values when semantic
analysis can determine the right-hand ADT descriptor and concrete payload
types. Literal patterns are match-only in the implemented slice; using one in a
`let` statement reports `pattern.refutable_let`.

A return may name the returned value for postconditions with `-> name: Type`.
The binding is contract-facing only: it is visible to `ensure` clauses for the
same function and to runtime `ensure` checks for tail-expression returns and
`?` early returns, but not to `require` clauses, the function body, or callers.
Bare `result` has no special meaning.

Selected package-relative `.veln` sources derive local module identity from
their selected source path. Path separators become `::`, so `foo.veln` derives
`foo`, and a `bar.veln` file below a `foo` directory derives `foo::bar`. Each
path segment must be a source module identifier. Invalid segments produce
`module.invalid_source_path`, and multiple selected source files deriving the
same module path produce
`module.duplicate_source_path`.

Source `mod` declarations are rejected with `module.source_mod`. Module paths
in `use` declarations use `::`; dotted module delimiters such as
`use foo.bar` are rejected with `module.invalid_import_path`.

When a project root contains `veln.toml`, the implemented manifest subset may
list package metadata, tool metadata, and public source-file exports:

```toml
[package]
name = "app"
description = "Example package."

[tool.docs]
format = "markdown"

[lib]
exports = ["src/main.veln"]

[dependencies."github.com/oakcask/lib"]
git = "https://example.invalid/lib.git"
tag = "v1.2.0"
subdir = "packages/lib"

[dependencies."github.com/oakcask/vendor-lib"]
vendor = "vendor/vendor-lib"
```

`[package]` stores string-valued package facts such as package identity,
version, description, or documentation pointers. `[tool.<name>]` stores
string-valued tool-specific facts. These fields are manifest-owned metadata
and are used by generated documentation. They do not create source symbols and
do not affect parsing, name resolution, type checking, lowering, or execution.

Dependency table keys are package identities used by
`use path from "package"` declarations. Path dependencies use a string-valued
`path` field. Vendor dependencies use a string-valued `vendor` field naming an
already available vendored package directory. Git dependencies use a
string-valued `git` field and exactly one selector field: `rev`, `tag`, or
`branch`. `subdir` is optional package-root metadata inside the selected git
source. Source analysis commands validate this metadata but do not fetch git
sources, resolve revisions, load vendor or mirror dependencies, or update
lockfiles. Mirror dependencies use a string-valued `mirror` field naming an
already materialized source tree.
`veln package lock` writes lockfiles for already available path dependencies,
vendor dependencies, mirror dependencies, and git dependencies with one
`rev`, `tag`, or `branch` selector, materializing non-local git URLs through
git when needed. Lockfile generation follows dependency manifests across the
graph and rejects incompatible source selections for repeated package
identities.

`[lib].exports` lists package-relative `.veln` source file paths. Export
entries must stay inside the package, use source-file spelling rather than
module-path spelling, derive a valid source module path, and match selected
source files. Duplicate export entries for the same derived module path are
reported. `[modules]` is rejected; manifests cannot rename source modules.

`use` declarations create module imports. `use foo::bar` resolves to the
selected source file deriving `foo::bar`; `use math` resolves to a selected
`math.veln` module. `use path from "package"` resolves `path` inside an
already available path dependency whose manifest package name matches
`package`, and the dependency module must be listed by that package's
`[lib].exports`. An import imports public functions by bare name and permits
qualified access through the written module path, such as `foo::bar::double()`
or `math::double()`. It does not create a short `bar::name` alias for
`use foo::bar`. Same-package qualified access requires a matching written
`use` declaration in the same source module. Every user module also has an
implicit standard `prelude` import. Public prelude helpers are available by
bare name under the same unambiguous import rule and by qualified paths such
as `prelude::vec_len(items)`. The `prelude` module name and import alias are
reserved for this standard import in user source. Public source ADT
constructors may also use the import path, either as `module::Constructor` or
`module::Type::Constructor`.

Public `fn` declarations, public source `type` declarations, and public member
aliases are the implemented source-level public API boundary. `[lib].exports`
is the implemented manifest-level package export list for public source
modules.
Function declarations can be referenced by bare name or by a `use`
alias-qualified path as callable values where a function-typed expression is
expected. When a selected `run` or `test` entry uses a function declaration as a
value, that referenced function is part of the selected executable slice. In a
named source module, selected-entry reachability treats a bare function
reference as a reference to the same source module. `use` alias-qualified
references keep the imported module identity. Bare local bindings, parameters,
and match-pattern bindings shadow same-named function declarations for this
reachability rule. Calls through a function-typed local binding or parameter
conservatively include visible function declarations with the same function
shape when surface reachability cannot prove one concrete declaration target.
Fixed-arity callables use the exact argument count. Variadic callables use the
same fixed parameter count and variadic element type, and a written call with
fewer than the fixed parameter count does not add variadic function targets.

A public member alias publishes an existing function, source ADT member, or
schema member through the declaring module's public path without introducing a
wrapper, a new type identity, a new schema identity, or a generated codec
alias:

```veln
pub fn parse = impl::parse
pub type Document = impl::Document
pub schema Packet = wire::Packet
```

The left side is the exported member name. The right side is a member path,
not an expression, signature, constructor list, or body. `pub fn` aliases
resolve to function members and `pub type` aliases resolve to source ADT
members. `pub schema` aliases resolve through schema-aware lookup to public
schema members or public schema aliases. Missing, private, function, source
ADT type, and codec targets are rejected at the alias declaration. A schema
alias does not import schema-local field names, generated helper names, codec
names, or ordinary source type bindings. An alias name shares the corresponding
function, type, or schema member namespace for the declaring module.

`test` is a top-level declaration keyword, not a visibility modifier. Test
declarations are selected by `veln test` from `*_test.veln` files, explicit
targets, and any automatically discovered source file that contains a top-level
`test` declaration. They require an empty parameter list and an explicit return
type. They may omit the declaration-level `effects [...]` clause for an empty
declared effect set, and the clause must be non-empty when written. They are
not ordinary callable functions.

## Documentation Comments And Doctests

Documentation line comments may contain executable doctest fences. A doc
comment fence whose info string is `veln` is extracted as generated test source
for `veln check` and `veln test`.

Executable doctest metadata is one concept with separate checks:

- `error=<TypePath>` makes the generated wrapper return
  `Result<(), <TypePath>>` and append an implicit `Ok(())` success value. If the
  fence omits `error=<TypePath>`, contains `?`, and immediately documents a
  public function with an explicit `Result<_, E>` return type, the generated
  wrapper uses `Result<(), E>` and also appends the implicit `Ok(())` success
  value. If there is no documented result context, the wrapper error type is
  inferred when every `?` applies to a known function call returning
  `Result<_, E>` and all such calls use the same `E`.
- `runtime=contract`, `clause=<Clause>`, and `predicate=<Predicate>` on a
  positive executable doctest expect a runtime contract failure. Optional
  `function=<Name>` and `blame=<Side>` attributes further constrain that
  runtime failure match.
- `runtime=ensure` and `predicate=<Predicate>` on a positive executable
  doctest expect a runtime `ensure` contract failure. Optional
  `function=<Name>` and `blame=<Side>` attributes further constrain that
  runtime failure match.
- `runtime=result` and `value=<FormattedValue>` on a positive executable
  doctest expect the generated test to return `Err(<FormattedValue>)`.
- `ignore` treats the fence as a documentation-only code example and does not
  create a generated doctest.
- `fail` marks a negative static example. It is checked as a generated private
  function and is accepted only when that generated source produces at least one
  error diagnostic. Hint-only diagnostics do not satisfy the expected failure.
  It is not selected as a runtime doctest case and cannot attach expected
  output.

Any other `veln` fence attribute reports `doctest.unknown_metadata`. Empty
runtime metadata values, unsupported runtime expectation kinds,
`runtime=contract` without both `clause` and `predicate`, `runtime=result`
without `value`, `runtime=ensure` without `predicate`, and an empty `error=`
value report
`doctest.invalid_metadata`.

An adjacent doc comment fence whose info string is
`veln-output stream=stdout` or `veln-output stream=stderr` attaches expected
output to the immediately preceding generated doctest.

Inside an executable `veln` fence, a line that starts with `> ` is hidden setup:
the generated test includes the line after removing the marker. `# comment`
remains visible example source and is included as a normal source comment. The
`> ` hidden marker is exact after the doc-comment prefix and one optional
separator space; an example that intentionally starts source with `>` can write
one extra leading space before `>`. Hidden setup is useful for imports,
helpers, and bindings that the documented sample should use without displaying
harness-only setup as example code.
Unknown `veln-output` attributes, missing `stream`, and stream values other
than `stdout` or `stderr` report doctest metadata diagnostics. A doctest may
attach at most one expected-output fence for each stream. A second
`veln-output` fence for the same stream reports
`doctest.duplicate_output` at the duplicate fence and leaves the first fence as
the selected expectation.

Documentation line comments may also contain schema references written as
`{@schema Name}` or `{@schema module::Name}`. Schema documentation references
resolve through the same schema-aware lookup used by codec declaration heads:
bare references resolve schemas and public schema aliases in the same module,
and qualified references require a matching written `use` path, including
nested module paths such as `use app::nested`, and a public schema or public
schema alias. Missing, private, function, source ADT type, and codec targets
are rejected with name diagnostics at the referenced name span.
Schema documentation references do not import schema-local field names, expose
generated helper names, expose codec names, or create ordinary source type
bindings. The generated Markdown documentation renders a resolved schema
reference as code text. The executable documentation cases
`../../examples/specification/doc/schema-references/` and
`../../examples/specification/doc/schema-reference-diagnostics/` pin the
accepted and rejected behavior.

Documentation line comments may also contain ADR-lite records. A complete
record starts with `## @adr` or `## @adr-lite` and then provides these fields
as `key: value` doc-comment lines: `id`, `status`, `scope`, `context`,
`decision`, and `consequences`.

```veln
## @adr
## id: module-boundary
## status: accepted
## scope: module
## context: Module identity is compiler-visible.
## decision: Keep the source path canonical.
## consequences: Manifest metadata cannot rename the module.
pub fn main() -> ()
	()
end
```

The parser exposes complete ADR-lite records as structured source metadata and
attaches each record to the nearest following public function declaration when
one exists. ADR-lite records are ignored for runtime semantics: they do not
affect parsing of declarations, type checking, lowering, execution, or
generated output.

## Expressions

Implemented expressions:

- holes: `_` and `_name`, with optional `satisfy candidate => predicate`
- literals: strings, integers, floats, `true`, `false`, and `()`
- paths and calls: `name`, `module::name`, `callee(args...)`
- type-applied call callees: `callee<TypeText>(args...)`
- callable function declaration values by bare name
- constructors: `Ok(value)`, `Err(error)`, `Some(value)`, `None`, `Nil`,
  `Cons(head, tail)`, and their `Result::`, `Option::`, or `List::`
  qualified forms
- channel effect calls: `channel::bounded(capacity)`,
  `channel::bounded<Item>(capacity)`, `channel::clone(tx)`,
  `channel::send(tx, value)`, `channel::recv(rx)`,
  `channel::select(left, right)`,
  `channel::select_priority(left, right)`,
  `channel::select_many_priority(receivers)`,
  `channel::select_many_timeout(receivers, timeout_ms)`,
  `channel::select_many_timeout_result(receivers, timeout_ms)`,
  `channel::select_many_timeout_cancellable(receivers, timeout_ms, token)`,
  `channel::select_timeout(left, right, timeout_ms)`,
  `channel::select_timeout_cancellable(left, right, timeout_ms, token)`,
  `channel::select_result(left, right)`,
  `channel::select_priority_result(left, right)`,
  `channel::select_timeout_result(left, right, timeout_ms)`, and
  `channel::close(tx)`
- task effect calls: `task::spawn(job)`, `task::spawn<Item>(job)`,
  `task::spawn_with(job, context)`,
  `task::spawn_with<Item>(job, context)`,
  `task::spawn_with<Item, Context>(job, context)`, `task::join(task)`,
  and `task::cancel(task)`
- network and time boundary calls: `net::receive_chunk()`,
  `net::send_chunk(bytes)`,
  `net::read_chunk_until_cancellable(stream, deadline, token)`,
  `net::write_chunk_until(stream, bytes, deadline)`,
  `net::write_chunk_until_cancellable(stream, bytes, deadline, token)`,
  `net::write_chunks_until(stream, chunks, deadline)`,
  `net::write_chunks_until_cancellable(stream, chunks, deadline, token)`,
  `net::shutdown_write(stream)`,
  `time::monotonic_ms()`,
  `time::timeout_ms(milliseconds)`,
  `time::deadline_after_ms(milliseconds)`, `time::wait_until(deadline)`,
  `time::cancel_token()`, `time::cancel_owner()`,
  `time::cancel_token_from(owner)`, `time::cancel_owned(owner)`,
  `time::cancel(token)`, and `time::is_cancelled(token)`,
  `time::wait_until_cancellable(deadline, token)`, and
  `time::wait_until_cancellable_outcome(deadline, token)`
- prelude helpers as bare or qualified calls such as `vec_len(items)` and
  `prelude::vec_len(items)`
- reserved embedded-standard-library builtin calls such as
  `prelude_builtin::vec_fold(items, initial, f)`
- records: `{name: value, ...}`
- dictionaries: `{key_expr: value_expr, ...}` when the first entry is not a
  bare `name: value` field; identifier-led expression keys such as `seed + 1`
  are dictionary keys
- record field access: `expr.name`
- vec literals: `[value, ...]`
- match expressions over literals, bindings, `_`, record patterns, and
  descriptor-backed constructors `Some`, `None`, `Ok`, `Err`, `Nil`, `Cons`,
  and their `Option::`, `Result::`, or `List::` qualified forms
- `if` / `else if` / `else` expressions with a required final `else`
- prefix operators: `not`, `-`
- pipelines: `expr |> target(args...)`
- binary operators: `or`, `and`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `+`, `-`,
  `*`, `/`
- postfix result propagation: `expr?`
- parenthesized expressions

`true` and `false` are boolean literals in expression and pattern positions;
they are not ordinary value names.

`Option` and `Result` constructors are built-in compiler-owned ADT
constructors. Source `type` declarations define additional ADT descriptors
with angle-bracket generic parameters, nullary variants, tuple-like variants,
and record-shaped variant declarations:

```text
pub type Maybe<A>
  pub Missing
  pub Just(A)
end
```

Constructors are value-level expressions. Nullary variants can be used as bare
names, and payload variants use call syntax such as `Just(1)` or
`Maybe::Just(1)`. In the declaring module, constructors resolve as bare names
or type-qualified names. From an importing module, public constructors also
resolve through the import alias as `alias::Constructor` or
`alias::Type::Constructor`. A public type does not automatically export its
constructors; each exported constructor line uses its own `pub` prefix.
Variant payload fields do not have independent visibility syntax. Private
constructors remain usable in their declaring module. One ADT cannot declare
the same constructor leaf name twice. Different ADTs in the same module may
reuse a constructor leaf name; bare use of that leaf is ambiguous and must use
a type-qualified path. When imports expose the same public constructor leaf
name, unqualified use is ambiguous and must use a qualifying path. The built-in
`List<A>` descriptor recognizes `Nil`, `Cons(head, tail)`, `List::Nil`, and
`List::Cons(head, tail)` and keeps the existing runtime list representation.
Type declarations use `type Name<A>` for declared type parameters. Legacy
`type Name(A)` declarations are rejected through ordinary parse recovery.

A `satisfy` suffix is valid only on a hole expression. The suffix requires one
candidate binding, the `=>` separator, and a predicate. The candidate binding
is visible only inside the suffix predicate.

Pipelines require a named or qualified call expression on the right. The piped
expression is inserted as the first argument of that call, so
`value |> target(extra)` is checked and executed as `target(value, extra)`. A
non-call target, or a call whose callee is not a name path, reports
`type.pipeline_target`.

Method-call-shaped syntax such as `value.method(args)` is parsed as a call
whose callee is a field access, but it is not a valid implemented call form.
The checker reports `type.method_call` at the method name and expects the
canonical named function-call spelling with the receiver passed explicitly.

Angle-bracket type-applied call callees currently contribute static item-type
information only for recognized built-in calls such as
`channel::bounded<String>(capacity)`. Square-bracket explicit type arguments
such as `channel::bounded[String](capacity)` are rejected through ordinary
parse recovery. Type-applied callees are not a general user-defined generic
function mechanism.

Call arguments must be separated with commas and closed with `)`. When the
parser can identify an adjacent argument without a separator, it reports
`parse.call_argument` and continues as if a comma had been inserted.

`match` is a primary expression and may appear anywhere an expression is
accepted, including call arguments and aggregate literals. Match arms are tried
in source order. The implemented match-pattern subset covers wildcard `_`,
binding names, literals, record patterns, built-in constructors, and
source-declared constructors in bare, type-qualified, alias-qualified, or
alias-and-type-qualified forms. Record patterns match when the scrutinee is a
record containing every named pattern field and every nested field pattern
matches. Pattern
bindings in one arm or `let` statement must not duplicate another binding in
that pattern or a value binding already visible at the pattern. Record pattern
field names must be unique.

The checker rejects non-exhaustive `match` expressions for scrutinee types it
can classify as finite domains: `Bool`, `Option<T>`, `Result<T, E>`, `List<A>`,
and source-declared ADTs. `_` and binding patterns are catch-all arms. Bool
matches must cover `true` and `false`; option matches must cover `Some(_)` and
`None`; result matches must cover `Ok(_)` and `Err(_)`; list matches must cover
`Nil` and `Cons(_)`; source-declared ADT matches must cover every declared
variant unless a catch-all arm is present.

`if` is a primary expression and may appear anywhere an expression is accepted.
Each `if` and `else if` condition follows the same Boolean checking rules as
the scrutinee in an equivalent `match Bool` expression. Branch result
expressions use the same expected-type and unification behavior as equivalent
`match Bool` arms. Every `if` expression requires a final `else` and closing
`end`; `else if` remains a chain branch for formatting rather than an `else`
body whose first expression is another `if`.
Checked evidence for successful `if` expressions, formatter preservation,
non-`Bool` condition diagnostics, branch result diagnostics, and parse
recovery for incomplete branches lives under
`../../examples/specification/check/if-expression-syntax/`,
`../../examples/specification/fmt/if-expression-syntax/`,
`../../examples/specification/check/if-expression-condition-diagnostics/`,
`../../examples/specification/check/if-expression-condition-human/`,
`../../examples/specification/check/if-expression-branch-diagnostics/`,
`../../examples/specification/check/if-expression-branch-human/`, and
`../../examples/specification/check/if-expression-parse-recovery-diagnostics/`.

## Contract Predicates

`require`, `ensure`, `invariant`, and hole `satisfy` predicates accept this
implemented syntax:

- literals, names, qualified names, and `()`
- grouping with parentheses
- plain or qualified call syntax
- field access syntax
- prefix `not` and `-`
- binary `or`, `and`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `+`, `-`, `*`, and
  `/`

The parser rejects holes, `?`, pipelines, `match`, records, and lists in
contract predicates before semantic checking. A syntactically valid predicate
may still fail contract validation. Function calls must resolve to discovered
pure functions or pure prelude helpers. Bare calls resolve against the current
program's function names or the implicit prelude import, and qualified calls
resolve through `use` aliases or `prelude::` for standard prelude helpers.
Call arguments must be assignable to declared parameter types. Function
declaration values may be passed to contract calls where the callee expects a
function type; bare references resolve against visible function declarations,
and `use` alias-qualified references keep the imported module identity. Numeric
return values from pure calls may be used inside
arithmetic operands of comparison predicates. Record-typed return values from
pure calls may feed field access, such as `summary(value).ready`. Field access
must resolve through record-typed values visible to the clause or returned by a
pure call.

Valid clauses are executable obligations. `require` is checked at function
entry. `ensure` is checked before an ordinary tail-expression return and may
read an explicit result binding. `invariant` is checked both at function entry
and before an ordinary tail-expression return or `?` early return; it uses the
same visible bindings as `require` and cannot read an explicit result binding.

## Not Implemented

Implemented lowering and execution do not include method calls, loops,
mutation, classes, traits, macros, comprehensions, anonymous functions, custom
operators, task selection, manifest fields beyond the implemented `[package]`,
`[tool.<name>]`, and `[lib].exports` subset, foreign declarations, or
doctest metadata other than `error`, `ignore`, `fail`, `runtime=contract`,
runtime contract detail attributes, `runtime=ensure`, runtime ensure detail
attributes, `runtime=result`, runtime result value matching, and
`veln-output` stream selection. Codec execution, generated decode or encode
functions, and `with` function signature checking are not implemented.
