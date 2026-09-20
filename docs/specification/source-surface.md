---
role: specification
authority: normative
specification-coverage: usage=#usage-and-declaration-forms; behavior=#schemas; limits=#diagnostics
update-when: Veln declarations, expressions, literals, schema syntax, companion sources, or executable source grammar changes.
---

# Source Surface

This page specifies implemented source syntax. The executable grammar in
[source-surface-executable.pl](source-surface-executable.pl) corroborates the accepted and rejected source forms.

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
  method-call diagnostics: this page.
- Contract predicate grammar: this page.
- Identifier casing for source-written module headers, ADT types,
  constructors, functions, tests, public aliases, bindings, parser recovery,
  and selected-command reachability:
  [names-effects.md](names-effects.md).
- Formatter layout and canonical comment spelling:
  [commands.md](commands.md).

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
the source-surface grammar artifact must agree with these productions.

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
Function      ::= "pub"? "fn" Name EffectBinder? "(" ParamList? ")" Return? Effects? NL
                  Contract* Body "end" NL?
TestDecl      ::= "test" Name "(" ")" Return Effects? NL
                  Contract* Body "end" NL?
TypeDecl      ::= "pub"? "type" Name TypeParamList? NL TypeVariant+ "end" NL?
EffectDecl    ::= "pub"? "effect" Name NL EffectOperation+ "end" NL?
EffectOperation ::= Name "(" EffectParamList? ")" "->" TypeText NL
EffectParamList ::= Name ":" TypeText ("," Name ":" TypeText)*
HandlerDecl   ::= "pub"? "handler" Name "(" ParamList? ")" "handles" MemberPath Effects? NL HandlerOperationClause+ "end" NL?
HandlerOperationClause ::= Name "(" HandlerOperationParams? ")" "=>" Expr NL
HandlerOperationParams ::= Name ("," Name)*
SchemaDecl    ::= "pub"? "schema" Name NL SchemaFormat? SchemaField+ SchemaValidation? "end" NL?
SchemaFormat  ::= "format" "binary" NL
SchemaField   ::= Name ":" SchemaFieldType SchemaFieldWhere? NL
SchemaFieldType ::= TypeText | LowercaseSchemaPrimitive | LowercaseReservedBitsPrimitive | ReservedBitsPrimitive | RepeatPrimitive | CanonicalRepeatPrimitive
LowercaseSchemaPrimitive ::= "uint" IntLiteral ("be" | "le")?
LowercaseReservedBitsPrimitive ::= "uint" IntLiteral ("be" | "le")? "reserves" IntLiteral
ReservedBitsPrimitive ::= "ReservedBits" "(" IntLiteral "," IntLiteral ")"
RepeatPrimitive ::= "Repeat" "(" CountExpr "," TypeText ")"
CanonicalRepeatPrimitive ::= "[" SchemaFieldType ";" CountExpr "]"
CountExpr ::= Name | Name ("-" | "+" | "*" | "/") Name
SchemaFieldWhere ::= "where" (ContractPredicate | ByteViewMultiplePredicate)
ByteViewMultiplePredicate ::= "payload_count" "multiple" "of" (Name | IntLiteral)
SchemaValidation ::= "validate" ContractPredicate NL
PublicAlias   ::= "pub" ("fn" | "type" | "schema") Name "=" MemberPath NL
TypeParamList ::= "<" Name ("," Name)* ","? ">"
EffectBinder  ::= "<" "effect" Name ">"
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
EffectList    ::= EffectEntry ("," EffectEntry)* ","?
EffectEntry   ::= MemberPath | "..." Name
Contract      ::= ("require" | "ensure" | "invariant") ContractPredicate NL
Body          ::= (LetLine | ExprLine)*
LetLine       ::= "let" LetPattern (":" TypeText)? "=" Expr NL
LetPattern    ::= "_" | BindingName | ConstructorPattern | RecordPattern
ExprLine      ::= Expr NL
Expr          ::= PrefixExpr (BinaryOp PrefixExpr)*
BinaryOp      ::= "|>" | "or" | "and" | "|" | "^" | "&" | "==" | "!="
                  | "<" | "<=" | ">" | ">=" | "<<" | ">>" | ">>>"
                  | "+" | "-" | "*" | "/"
PrefixExpr    ::= ("not" | "-" | "~") PrefixExpr | PostfixExpr
PostfixExpr   ::= PrimaryExpr (Call | TypeArgs | FieldAccess | "?")*
PrimaryExpr   ::= Hole | Literal | NamePath | Perform | Handle | SchemaDecode | SchemaEncode | "(" Expr ")" | "()"
                  | Record | Dict | List | Match | If
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
