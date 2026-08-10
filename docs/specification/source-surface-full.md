---
role: specification
authority: normative
update-when: The source grammar or executable source-surface fixtures change.
---

# Source Surface Details

Use [source-surface.md](source-surface.md) first. The executable grammar in
[source-surface-executable.pl](source-surface-executable.pl) and checked cases
under `../../examples/specification/` are the primary source-surface evidence.

## Current Schema Boundary

Schemas contain fields, an optional `format binary` clause, field-local
`where` predicates, and at most one schema-level `validate` predicate.
Explicit schema `decode` and `encode` expressions use schema-local visible
records. Compatibility helper lowering uses the same record boundary. Domain
projection is ordinary source code outside the schema body.

Schema-level mapping clauses are rejected by the parser and are not part of
the implemented grammar.

Top-level `codec` and `pub codec` declarations are rejected by the parser and
are not part of the implemented grammar. Source code uses ordinary functions
plus explicit schema `decode` and `encode` expressions for executable decode
and encode entry points.

`format binary` dispatch payload cases accept lowercase exact-width `uint...`
primitive spelling in the same positions as compatible upper-case exact-width
primitive payload spelling. They also accept
byte-aligned lowercase `uint... reserves <value>` spelling and subbyte
spellings from `uint1 reserves 0` through `uint7 reserves 127` in direct
reserved-bit dispatch payload positions when the reserved value fits the
declared width.
`format binary` direct nested schema fields may name an eligible same-module
schema or public imported schema and expose the nested schema-local visible
record at that field.
Format-neutral schemas reject binary-only primitive vocabulary in dispatch
payload positions. Format-neutral generated decode helpers are limited to
recursive visible shapes made from scalar leaves, anonymous record fields,
`Option<T>`, `List<T>`, `Vec<T>`, `Dict<String, T>`, and `Result<Ok, Err>` when both
payloads are recursive visible shapes. Same-module source ADTs and public
imported source ADTs referenced through written `use` paths are supported in
those positions when every constructor payload is a recursive visible shape.
Format-neutral generated encode helpers use the same visible-shape vocabulary:
`Int`, `Bool`, `Float`, and `String` leaves, anonymous records,
`Option<T>`, `List<T>`, `Vec<T>`, `Dict<String, T>`, `Result<Ok, Err>`, and
eligible same-module or public imported source ADTs. Every recursively visited
child or constructor payload must also be eligible; container depth is not
otherwise limited. Decode may stop when it sees the same source ADT descriptor
again with changed type arguments. Encode instead checks type arguments newly
introduced at that recursive edge and rejects unsupported leaves in them.

## Executable Grammar

<!-- source-surface-grammar:start -->
```text
Module        ::= UseDecl* Item*
UseDecl       ::= "use" ModulePath ImportSource? NL
ImportSource  ::= "from" PackageString
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
LetPattern    ::= "_" | BindingName | RecordPattern
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

## Read When

- Use this page only as a stable route for old links.
- Prefer the executable grammar and focused checked examples for current
  syntax details.
