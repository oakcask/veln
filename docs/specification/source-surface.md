---
role: specification
authority: normative
specification-coverage: usage=#usage-and-declaration-forms; behavior=#schemas; limits=#diagnostics
update-when: Veln declarations, expressions, literals, schema syntax, companion sources, or executable source grammar changes.
---

# Source Surface

This page specifies implemented source syntax. The executable grammar in
[source-surface-executable.pl](source-surface-executable.pl) and the checked
parser and command cases corroborate the accepted and rejected source forms.

## Usage and declaration forms

This representative shape shows imports, a public effect row, and binary schema fields:

```veln
use stdio

pub fn greet(name: String) -> () effects [stdio]
	stdio::println(name)
end

schema Packet
	format binary
	length: UInt16be
	kind: UInt8
end
```

The sections below explain declarations and source boundaries. The grammar
provides the production notation for these forms.

### Declaration and expression inventory

- Optional source-written `mod` headers, source path derived local module
  identity, local and external package imports, `.test.veln` test companion
  source classification, functions with optional `<effect E>` row binders,
  tests, source ADT type declarations, schema declarations, nominal effect
  operation declarations, lexical handler declarations, public member
  aliases, canonical `#` comments, `##`
  documentation comments, doctests, ADR-lite metadata, and manifest dependency
  metadata plus `[lib].exports` source-file exports: this page.
- Expression forms, constructors, records, dictionaries, vecs, matches,
  `if` / `else if` / `else` expressions, pipelines, ordinary and variadic
  calls, function type effect rows with final `...E` tails, `perform`
  operation expressions, `handle ... with ...` expressions, standard channel
  calls, zero-argument task spawns, one-context `task::spawn_with` calls, and
  method-call diagnostics, and the source boundary of executable `begin` and
  `defer` forms: this page.
- Contract predicate grammar: this page.
- Call-site-aware function declarations and their built-in local:
  [call-site-declarations.md](call-site-declarations.md).
- Identifier casing for source-written module headers, ADT types,
  constructors, functions, tests, public aliases, bindings, parser recovery,
  and selected-command reachability:
  [names-effects.md](names-effects.md).
- Formatter layout and canonical comment spelling:
  [commands.md](commands.md).

### Cleanup-region forms

`begin` is a value-producing expression whose body introduces a lexical scope.
Its body accepts the same direct body lines as a function or test. A `defer`
statement is a direct body line of a function, test, or `begin`; its own block
therefore also parses as a body. A `defer` token in an expression position
reports `parse.expected_expression` at that token. Recovery skips the invalid
token without consuming a following declaration boundary. Both forms require
a closing `end`. A missing closing delimiter reports `parse.begin_missing_end`
or `parse.defer_missing_end`. Recovery leaves an enclosing declaration's
closing `end` available to that declaration. Within an `if` or `match`, it also
leaves the following `else` branch or match arm available to the enclosing
expression. It preserves the next top-level declaration as a separate item,
including when nested cleanup forms are both unterminated. The lossless tree
retains every source token. At most 128 `begin` and `defer` cleanup forms may be
nested in total; both forms share this limit. The next level reports
`parse.cleanup_nesting_limit` without aborting lossless-tree construction.

The [type rules](types.md#inference-rules), [effect boundary](effects.md#effect-labels),
and [binding visibility](name-resolution.md#value-calls-and-shadowing) are
specified by their focused pages. Formatter behavior is specified by the
[format command](command-fmt.md#formatting-rules). LSP and MCP navigation are
specified by [editor support](editor-support.md#lsp-navigation-formatting-and-rename)
and [saved workspace navigation](mcp.md#saved-workspace-navigation).

These forms have a public static, tooling, and executable surface. The
[execution boundary](execution.md#runtime-readiness-and-host-boundaries)
specifies cleanup ordering, failure precedence, task cancellation, and runtime
limits.

### Variant-refinement-shaped type text

Type positions recognize a structural variant alternative as a named ADT base
whose leaf starts with an ASCII uppercase letter, optional type arguments, and
an `::` plus an ASCII-uppercase final segment. Qualifier segments before the
base leaf do not have a casing restriction. A `|` joins complete alternatives
into one structural union:

```veln
fn transition(
	state: protocol::State::Ready | protocol::State::Closed,
) -> Result<Int, Error>::Ok
	state
end
```

The same structure is recognized in function, test, effect-operation, and
handler parameter or return types; ADT payload and schema fields; local
annotations; nested record and function types; generic arguments; and explicit
call type arguments. The syntax tree and lowered AST preserve the written base
paths, type arguments, final segments, alternative order, duplicates, and
source spans. Encoding and decoding the lowered AST through its wire format
preserves that structure and those spans.

Type arguments belong before the final segment. Each side of `|` must be a
complete structural alternative, and `|>` remains the pipeline token rather
than a type separator. In a non-generic multi-segment spelling, the last
segment is the final variant and every preceding segment is the qualified base.
An otherwise ordinary qualified generic type such as
`Alias::Container<Int>` remains a named type for later name and casing
analysis. A path such as `protocol::state::Ready`, whose prospective base leaf
is not uppercase, also remains named type text for later name and casing
analysis rather than becoming refinement syntax. Once type arguments before
`::Variant` make the refinement structure explicit, further type arguments
after that final segment are malformed.

Adjacent closing angle brackets close the innermost type arguments first.
Thus `sink<List<State::Ready>>(value)` is valid: the first `>` closes `List`
and the second closes the call's explicit type arguments. In
`sink<State::Ready>>(value)`, the second `>` is surplus and remains part of the
recovered source.

#### Malformed variant-refinement forms

The parser reports `parse.variant_refinement_type` for these malformed forms:

- an incomplete union alternative;
- a missing base or final segment;
- a lowercase final segment;
- an empty or misplaced type argument;
- a missing generic closer;
- a surplus generic closer before or after the final variant;
- a segment following a generic base's final variant;
- adjacent type text outside the completed structural position; or
- `|>` between alternatives.

Nesting beyond 256 containing generic-argument boundaries reports the same
diagnostic and does not prevent lossless-tree construction.

Structural recognition does not prove that a base names an ADT, that its final
segment names a variant, or that union alternatives name the same instantiated
ADT. The [type-system limits](types.md#compatibility-and-limits) distinguish
this parser contract from semantic variant-refinement support.

## Test companion sources

A path ending exactly in `.test.veln` is a test companion. Its target is the
same-directory path formed by removing `.test`. It has its own path-derived
module identity. Chained names such as `math.test.test.veln` are invalid and
`_test.veln` integration modules remain ordinary sources. Documentation
selection excludes exact companions, whether discovered or explicitly named.

The target must exist in the same package for `check` and `test`. With an
explicit `use` of that exact target, a companion may qualify private target
functions, source ADT types and constructors, schemas in schema positions,
nominal effects in effect positions, and handlers in `handle` expressions.
The permission is exact-target and non-transitive: bare names, wrong targets,
missing imports, integration modules, and external packages do not gain it.

Companions cannot declare public functions, effects, handlers, types, public
variants, schemas, or public aliases. A manifest `[lib].exports` cannot publish
a companion. Top-level `codec` declarations remain rejected by
`parse.codec_declaration_removed` before companion validation. Missing, chained,
public-declaration, and manifest-export boundaries are diagnosed before
execution. Public declarations report `module.companion_public_declaration`.

## Integer Literals

`Int` literals accept decimal digits, lowercase `0b` plus binary digits, or
lowercase `0x` plus mixed-case hexadecimal digits. All three spellings use the
same nonnegative range through `9223372036854775807` and compare or match by
value. Leading zeroes, radix prefixes, and hexadecimal digit case are retained
by formatting.

Malformed prefixed candidates remain one token. Missing or invalid digits,
uppercase prefixes, separators, prefixed float forms, and out-of-range values
produce one `parse.integer_literal` diagnostic at the failed source fact. The
same rule applies in expression, pattern, schema, formatter, and diagnostic
contexts.

## Integer Bitwise Tokens

Expressions accept unary `~` and binary `&`, `|`, `^`, `<<`, `>>`, and
`>>>`. Lexing chooses the longest token, preserving `|>` as pipeline and
distinguishing `>>>`, `>>`, `>=`, and `>`. Adjacent closing angles in nested
generic types remain type delimiters rather than shift expressions.

## Schemas

A schema declaration is `schema Name ... end` or `pub schema Name ... end`.
A single `format binary` clause, when present, precedes fields. Without that
clause, fields use format-neutral types. A field has `name: Type` and may have a
field-local `where` predicate; one schema-level `validate` predicate follows
the fields.

### Format-neutral fields

Decode and encode expose a schema-local visible record only when every
recursively visited field and constructor payload is an eligible visible shape.
The scalar leaves are `Int`, `Bool`, `Float`, and `String`. Containers are
anonymous records, `Option<T>`, `List<T>`,
`Vec<T>`, `Dict<String, T>`, and `Result<Ok, Err>`. Same-module source ADTs
and public imported source ADTs are eligible when every constructor payload also
has an eligible shape. Private imports, missing or wrong-kind targets, and
unsupported ADT payloads are declaration errors. Decode and encode share this
vocabulary but differ in recursive generic stopping: decode may accept a
repeated source ADT descriptor with changed instantiated arguments; encode
checks newly introduced arguments and rejects unsupported leaves. There is no
separate container-depth limit.

### Binary fields

Binary fields use lowercase unsigned primitives such as `uint16be`,
compatibility spellings such as `UInt16be`, and reserved-bit forms `uint... reserves <value>`
and `ReservedBits(width, value)`. They also support legacy
`Repeat(count, Payload)`, canonical `[Payload; count]`, nested binary schemas,
recursive anonymous records whose leaves are exact-width unsigned primitives,
`ByteView(length)`, and closed or extension dispatch. Binary-only forms are
available only in `format binary` schemas.

A canonical repeat writes the payload type before `;` and the count after it.
The count may be an earlier visible count field or implemented arithmetic over
earlier count fields. Payloads may be exact-width primitives, lowercase
compatibility primitives, nested schemas, `ByteView(length_field)`, or
`ByteView(left_length - right_length)`. Repeated and dispatch payloads accept
the same lowercase primitive and supported reserved-bit spellings as direct
fields. Direct dispatch accepts subbyte `uint1 reserves 0` through
`uint7 reserves 127` when the value fits. `veln fmt` canonicalizes supported
compatibility spellings to lowercase schema vocabulary.

### Composition, records, and references

Type text resolves ordinary type and schema namespaces independently. A unique
schema or schema alias composes its schema-local record under the written field
binding; target fields are not injected as unqualified names. Same-module
private/public targets and public imported targets or aliases are supported.
Full imported paths take precedence over an implicit leaf alias. Colliding
workspace leaf aliases remain unresolved regardless of import order. Format-
neutral schemas may compose only format-neutral targets; binary schemas only
binary targets. Missing, private, wrong-kind, ambiguous, incompatible, cyclic,
duplicate-binding, forward-reference, and direction-specific helper failures
are declaration errors.

Anonymous binary records expose nested schema-local records when every leaf is an
exact-width unsigned primitive. Sibling nested records at one level are allowed.
Later repeat counts, byte-view lengths, dispatch tags or lengths, field
predicates, and schema validation may refer to an earlier decoded visible
`Int`, including a composed path such as `header.length`. A root binding must
already be decoded.

### Schema operations

`decode SchemaName from view at base_offset` accepts an eligible binary schema,
`ByteView`, and `ByteOffset`, and returns `DecodeStep<T>` for the schema-local
record. `encode SchemaName from value` accepts that record and returns
`Result<ByteChunk, EncodeError>`. For format-neutral schemas the operations
use `Result<T, String>` and do not produce binary bytes. Bare operation paths
resolve only to same-module schemas; qualified public schemas and public aliases
are available through written imports. A written import does not create a bare
schema operation path. Generated helper names are implementation details.

## Codecs

Top-level `codec Name for Schema directions...` and
`pub codec Name for Schema directions...` declarations are not accepted source
syntax. The parser reports `parse.codec_declaration_removed` at the `codec`
token and directs source toward ordinary functions plus explicit
`decode Schema from view at base_offset` and `encode Schema from value`
expressions.

## Diagnostics

Schema-level `map to` clauses, selected mappings, mapping assignments, and
`inverse` projection annotations are not source syntax; the parser reports
`parse.schema_mapping_removed` at `map`. Top-level `codec` declarations report
`parse.codec_declaration_removed` at `codec` and callers use ordinary functions
with explicit schema operations instead.

Schema diagnostics cover format placement, primitive kind, field and schema
predicates, dispatch payload eligibility, schema-path resolution, and helper
availability. References for repeat counts, `ByteView` lengths, dispatch tags,
and extension-dispatch tags or lengths must name earlier decoded visible
`Int` fields in the same schema. Invalid references report
`schema.repeat_reference`, `schema.byte_view_reference`, or
`schema.dispatch_reference` at the failed reference.

## Executable Grammar

The grammar below is the compact source contract. Parser implementation and
the source-surface grammar artifact must agree with these productions. The
productions describe accepted form; the cleanup nesting resource limit is the
prose contract above.

A handler declaration requires `for` between its parameter list and its one
nominal effect target. The former `handles` spelling is an ordinary identifier,
so it may be used wherever the corresponding identifier casing rules permit.
Using it as the handler separator is rejected with `for` as the expected token.

<!-- source-surface-grammar:start -->
```text
Module        ::= ModuleHeader? UseDecl* Item*
ModuleHeader  ::= "mod" ModuleHeaderPath NL
UseDecl       ::= "use" ModulePath ImportSource? NL
ImportSource  ::= "from" PackageString
ModuleHeaderPath ::= ModuleHeaderSegment ("::" ModuleHeaderSegment)*
ModuleHeaderSegment ::= Name | HoleName
HoleName      ::= "_" identifier-continue+
ModulePath    ::= Name ("::" Name)*
PackageString ::= String
IntLiteral    ::= DecimalLiteral | BinaryLiteral | HexadecimalLiteral
DecimalLiteral ::= ASCII decimal digit+
BinaryLiteral ::= "0b" ("0" | "1")+
HexadecimalLiteral ::= "0x" ASCII hexadecimal digit+
Item          ::= Function | TestDecl | EffectDecl | HandlerDecl | TypeDecl | SchemaDecl | PublicAlias
Function      ::= "pub"? "fn" Name EffectBinder? "(" ParamList? ")" Return? Effects? CallsiteModifier? NL
                  Contract* Body "end" NL?
TestDecl      ::= "test" Name "(" ")" Return Effects? NL
                  Contract* Body "end" NL?
TypeDecl      ::= "pub"? "type" Name TypeParamList? NL TypeVariant+ "end" NL?
EffectDecl    ::= "pub"? "effect" Name NL EffectOperation+ "end" NL?
EffectOperation ::= Name "(" EffectParamList? ")" "->" TypeText NL
EffectParamList ::= Name ":" TypeText ("," Name ":" TypeText)*
HandlerDecl   ::= "pub"? "handler" Name "(" ParamList? ")" "for" MemberPath Effects? NL HandlerOperationClause+ "end" NL?
HandlerOperationClause ::= Name "(" HandlerOperationParams? ")" "=>" Expr NL
HandlerOperationParams ::= Name ("," Name)*
SchemaDecl    ::= "pub"? "schema" Name NL SchemaFormat? SchemaField+ SchemaValidation? "end" NL?
SchemaFormat  ::= "format" "binary" NL
SchemaField   ::= Name ":" SchemaFieldType SchemaFieldWhere? NL
SchemaFieldType ::= TypeText | LowercaseSchemaPrimitive | LowercaseReservedBitsPrimitive | ReservedBitsPrimitive | ByteViewPrimitive | RepeatPrimitive | CanonicalRepeatPrimitive | DispatchPrimitive | ExtensionDispatchPrimitive
LowercaseSchemaPrimitive ::= "uint" IntLiteral ("be" | "le")?
LowercaseReservedBitsPrimitive ::= "uint" IntLiteral ("be" | "le")? "reserves" IntLiteral
ReservedBitsPrimitive ::= "ReservedBits" "(" IntLiteral "," IntLiteral ")"
ByteViewPrimitive ::= "ByteView" "(" CountExpr ")"
RepeatPrimitive ::= "Repeat" "(" CountExpr "," SchemaFieldType ")"
CanonicalRepeatPrimitive ::= "[" SchemaFieldType ";" CountExpr "]"
DispatchPrimitive ::= "Dispatch" "(" SchemaFieldReference ("," SchemaFieldReference)? "," DispatchCases ")"
ExtensionDispatchPrimitive ::= "ExtensionDispatch" "(" SchemaFieldReference "," SchemaFieldReference "," DispatchCases ")"
DispatchCases ::= IntLiteral "=>" SchemaFieldType ("," IntLiteral "=>" SchemaFieldType)*
CountExpr ::= IntLiteral | SchemaFieldReference | SchemaFieldReference ("-" | "+" | "*" | "/") SchemaFieldReference
SchemaFieldReference ::= Name ("." Name)*
SchemaFieldWhere ::= "where" (ContractPredicate | ByteViewMultiplePredicate)
ByteViewMultiplePredicate ::= "payload_count" "multiple" "of" (Name | IntLiteral)
SchemaValidation ::= "validate" ContractPredicate NL
PublicAlias   ::= "pub" ("fn" | "type" | "schema") Name "=" MemberPath NL
TypeParamList ::= "<" Name ("," Name)* ","? ">"
TypeText      ::= VariantRefinementType | NamedType | UnitType | RecordType | FunctionType
NamedType     ::= TypePath NamedTypeArguments?
UnitType      ::= "(" ")"
RecordType    ::= "{" RecordTypeFields? "}"
RecordTypeFields ::= RecordTypeField ("," RecordTypeField)* ","?
RecordTypeField ::= FieldName ":" TypeText
FieldName      ::= Name | "effect"
FunctionType  ::= "fn" "(" FunctionTypeParams? ")" "->" TypeText Effects?
FunctionTypeParams ::= FunctionTypeParam ("," FunctionTypeParam)* ","?
FunctionTypeParam ::= Name ":" TypeText | TypeText | "..." TypeText?
VariantRefinementType ::= VariantAlternative ("|" VariantAlternative)*
VariantAlternative ::= NamedAdtBase RefinementTypeArguments? "::" UpperName
NamedAdtBase  ::= (Name "::")* UpperName
NamedTypeArguments ::= "<" TypeText ("," TypeText)* ","? ">"
RefinementTypeArguments ::= "<" TypeText ("," TypeText)* ">"
TypePath      ::= Name ("::" Name)*
EffectBinder  ::= "<" "effect" Name ">"
TypeVariant   ::= "pub"? UpperName TypeVariantFields? NL
TypeVariantFields ::= "(" TypeVariantField ("," TypeVariantField)* ","? ")"
                  | "{" TypeVariantField ("," TypeVariantField)* ","? "}"
TypeVariantField ::= Name ":" TypeText | TypeText
ParamList     ::= Param ("," Param)* ","?
Param         ::= Name (":" (VariadicMarker TypeText? | TypeText))?
VariadicMarker ::= "..."
Return        ::= "->" ResultBinding? TypeText
ResultBinding ::= Name ":"
Effects       ::= "effects" "[" EffectList? "]"
CallsiteModifier ::= "callsite"
EffectList    ::= EffectEntry ("," EffectEntry)* ","?
EffectEntry   ::= MemberPath | "..." Name
Contract      ::= ("require" | "ensure" | "invariant") ContractPredicate NL
Body          ::= (LetLine | DeferStatement | ExprLine)*
LetLine       ::= "let" LetPattern (":" TypeText)? "=" Expr NL
DeferStatement ::= "defer" NL Body "end" NL?
LetPattern    ::= "_" | BindingName | ConstructorPattern | RecordPattern
ExprLine      ::= Expr NL
Expr          ::= PrefixExpr (BinaryOp PrefixExpr)*
BinaryOp      ::= "|>" | "or" | "and" | "|" | "^" | "&" | "==" | "!="
                  | "<" | "<=" | ">" | ">=" | "<<" | ">>" | ">>>"
                  | "+" | "-" | "*" | "/"
PrefixExpr    ::= ("not" | "-" | "~") PrefixExpr | PostfixExpr
PostfixExpr   ::= PrimaryExpr (Call | TypeArgs | FieldAccess | "?")*
PrimaryExpr   ::= Hole | Literal | NamePath | Perform | Handle | SchemaDecode | SchemaEncode | BeginExpr | "(" Expr ")" | "()"
                  | Record | Dict | List | Match | If
BeginExpr     ::= "begin" NL Body "end"
SchemaDecode  ::= "decode" MemberPath "from" Expr "at" Expr
SchemaEncode  ::= "encode" MemberPath "from" Expr
Perform       ::= "perform" MemberPath "::" Name "(" ArgList? ")"
Handle        ::= "handle" Expr "with" MemberPath "(" ArgList? ")"
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

## References

- Grammar artifact: [source-surface-executable.pl](source-surface-executable.pl).
- Parser: `crates/veln-syntax/src/parser/` and `crates/veln-syntax/src/lexer.rs`.
- Source identity and companion visibility: `crates/veln-analysis/src/surface/`.
## Read by task

- Updating parser behavior, AST source shape, source metadata, declaration
  rules, or formatter output.
- Checking whether a syntax feature is implemented rather than proposed.
- Aligning examples, diagnostics, or command behavior with accepted source
  syntax.

## Skip unless needed

- Do not read proposal or phase history before this page.
- Use [source-decisions.md](source-decisions.md) only when rationale is needed
  after the implemented source behavior is clear.
