---
role: specification
authority: normative
specification-coverage: usage=#usage-and-declaration-forms; behavior=#schemas; limits=#diagnostics
update-when: Veln declarations, expressions, literals, schema syntax, companion sources, executable source grammar, source formatting, syntax-tree or lowered-AST structural preservation, or the AST wire layout changes.
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

An import can bind an explicit module qualifier with `as` before an optional
package source:

```veln
use transport::net as wire from "std"
```

`as` is contextual in an import and remains an ordinary identifier elsewhere.
The alias is one identifier and must start with an ASCII lowercase letter.
The parser retains uppercase and underscore-led aliases for casing diagnostics.
The contextual spellings `callsite`, `handler`, and `handle` are valid explicit
aliases. When one of those spellings is followed by `::`, the parser retains a
qualified name path. In particular, `handle::member()` is a call path rather
than the start of a `handle ... with ...` expression.

### Import alias preservation and formatting

A valid explicit import retains the written target, the explicit alias and its
span, and the optional target package source. The lowered AST also retains the
package that owns the importing source; that origin remains distinct from an
explicit target package. Formatting writes the alias before `from`. A second
formatting pass produces the same source. The syntax tree and lowered AST also
retain the complete declaration span and recovery state. The AST wire layout
serializes that provenance and rejects the preceding layout header, whose
declaration records lack their owner-package fields. Importing-package
provenance was introduced by the earlier wire layout and remains preserved.

Lowered functions, handlers, module values, effects, source types, schemas,
and public aliases retain both their declaring package and module identity.
The AST wire form preserves both identities so declarations from different
packages do not become one declaration merely because their module and name
spellings match.

### Import alias recovery limits

A malformed alias clause recovers at its declaration newline, so the next
import or item remains available, but the recovered import supplies no lookup
binding. Semantic analysis continues after that bounded recovery so retained
target and alias spellings can report casing diagnostics; the recovered import
still cannot contribute a qualifier, exact path, or bare export. Formatting
preserves the malformed declaration text. It does not turn a recovered import
into a valid binding. Reparsing formatted output retains the recovered state,
and another formatting pass is unchanged.

Recovery retains only the provenance parsed before the malformed remainder:

| Malformed declaration shape | Retained explicit alias | Retained package source |
| --- | --- | --- |
| `use a as` | Absent | Absent |
| `use a as from "pkg"` | Absent | `pkg` |
| `use a as "local"` | Absent | Absent |
| `use a as b::c` | `b` | Absent |
| `use a from "pkg" as local` | Absent | `pkg` |
| `use a as b as c` | `b` | Absent |
| `use a as b from` | `b` | Absent |

Missing aliases, non-identifier aliases, trailing alias paths, misplaced or
repeated `as`, and missing package strings all follow this newline-bounded,
non-binding rule. A retained alias or package is diagnostic provenance only.
If a syntax tree contains any recovered import, formatting returns the complete
source text unchanged. This whole-source preservation prevents formatting of
other declarations from turning retained recovery provenance into a binding.

A source `type` declaration may contain zero variants:

```veln
type Empty
end
```

This form declares a named source type without declaring a constructor. It is
valid source syntax, but it cannot supply a variant to a refinement such as
`Empty::Missing`. The [type rules](types.md#compatibility-and-limits) and
[diagnostic contract](diagnostics-json.md#diagnostic-families) define that
semantic boundary.

### Module immutable value declarations

A module item may declare a typed immutable value with
`[pub] let name: Type = initializer`. The annotation and initializer are
required, and the declaration has no closing `end`. A physical newline or the
end of the source terminates the declaration. A following module item requires
a physical newline. The name must be one ordinary identifier that starts with
an ASCII lowercase letter. Contextual and reserved spellings, including
`callsite`, `handle`, `handler`, `true`, and `false`, are rejected. A wildcard,
hole name, destructuring pattern, constructor pattern, or declaration type
parameter is rejected. Recovery preserves the next module item after any of
these failures.

The initializer uses the same expression continuation and record-literal
syntax as expressions in function bodies. Delimited records, lists, and calls
may therefore span lines. The formatter writes the declaration at module
indentation and applies the ordinary canonical type and expression layout.

The syntax tree and lowered AST preserve the declaration's visibility, name,
written annotation, exact annotation span, structured type paths and
refinements, initializer, and source spans. The lowered AST also records the
declaring package and module identity. The AST wire layout serializes the
annotation span and both identities.
Incompatible older layout headers are rejected rather than decoded with an
older module-value record shape.

This source surface does not make module values executable. Semantic analysis
checks the written annotation and finite-data initializer, resolves
same-module and imported references, enforces package-aware declaration
identity and visibility, and rejects initializer cycles across package sources.
`veln fmt` accepts and formats a valid declaration, while `veln check` and
commands that require checked code still report `module.value_not_executable`.
Checked core and typed IR retain declarations, finite initializer trees, and
selected package-aware reference identities. Runtime materialization and code
generation are not implemented.

### Declaration and expression inventory

- Optional source-written `mod` headers, source path derived local module
  identity, local and external package imports, `.test.veln` test companion
  source classification, functions with optional `<effect E>` row binders,
  tests, source ADT type declarations, schema declarations, nominal effect
  operation declarations, lexical handler declarations, public member
  aliases, canonical `#` comments, `##`
  documentation comments, doctests, ADR-lite metadata, and manifest dependency
  metadata plus `[lib].exports` source-file exports: this page.
- Private and public typed module immutable value declarations, including
  multiline initializer syntax and malformed-declaration recovery: this page.
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

After a missing cleanup `end`, recovery preserves a following private typed
module `let` as a separate item regardless of indentation when its initializer
is followed by another top-level item or reaches the end of the source without
just closing a block. A private typed `let` remains local when its initializer
is followed by the enclosing declaration's `end`, or when the source ends
immediately after the initializer closes a block. A following public typed
module `let` remains a separate item. A malformed public module `let` candidate
also remains a separate item, reports its own declaration diagnostic, and does
not consume the next module item. After a properly closed cleanup region, a
following typed or untyped local `let` remains in the enclosing function or
test body instead of being promoted to a module item.

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

Type positions recognize a structural variant alternative as a named ADT base,
optional type arguments, and an `::` plus an ASCII-uppercase final segment.
Without type arguments, the base leaf must start with an ASCII uppercase
letter. Type arguments make the refinement shape explicit, so a generic base
leaf with other casing is preserved for semantic casing recovery instead of
being discarded as ordinary named-type text. Qualifier segments before the
base leaf do not have a structural casing restriction. A `|` joins complete
alternatives into one structural union:

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

For each named or positional ADT payload field, the syntax tree and lowered
AST preserve a type span that selects only the written type text. The complete
field span remains separate. The current wire layout serializes both spans and
rejects the preceding layout header instead of decoding bytes with the older
field shape.

Type arguments belong before the final segment. Each side of `|` must be a
complete structural alternative, and `|>` remains the pipeline token rather
than a type separator. In a non-generic multi-segment spelling, the last
segment is the final variant and every preceding segment is the qualified base.
An otherwise ordinary qualified generic type such as
`Alias::Container<Int>` remains a named type for later name and casing
analysis. A non-generic path such as `protocol::state::Ready`, whose
prospective base leaf is not uppercase, also remains named type text rather
than becoming refinement syntax. In contrast,
`protocol::state<Int>::Ready` is preserved as a structural refinement so later
analysis can report and recover the invalid base-leaf casing. Once type
arguments before `::Variant` make the refinement structure explicit, further
type arguments after that final segment are malformed.

A non-generic path whose prospective base and final segments both start with
ASCII lowercase letters, such as `state::ready`, remains ordinary named-type
text in the syntax tree. The parser also preserves it as a resolution-dependent
refinement candidate. The parser does not report a refinement diagnostic for
that candidate. If semantic analysis selects a uniquely initial-case-recovered
base, the candidate becomes an invalid refinement and the
diagnostic pipeline emits `parse.variant_refinement_type` before its semantic
diagnostics. Without that base selection, the path remains an ordinary named
type and does not acquire a refinement diagnostic.

Adjacent closing angle brackets close the innermost type arguments first.
Thus `sink<List<State::Ready>>(value)` is valid: the first `>` closes `List`
and the second closes the call's explicit type arguments. In
`sink<State::Ready>>(value)`, the second `>` is surplus and remains part of the
recovered source.

#### Malformed variant-refinement forms

The parser reports `parse.variant_refinement_type` for these structurally
selected malformed forms:

- an incomplete union alternative;
- a missing base or final segment;
- a lowercase final segment;
- an empty or misplaced type argument;
- a missing generic closer;
- a surplus generic closer before or after the final variant;
- a segment following a generic base's final variant;
- adjacent type text outside the completed structural position; or
- `|>` between alternatives.

The parser preserves a lossless tree for diagnostics and formatting recovery.
A structurally complete refinement whose final segment starts with an ASCII
lowercase letter also retains its refinement metadata for semantic diagnostic
recovery. The lowercase segment remains rejected syntax, including when the
base has type arguments, and recovery cannot make the annotation valid. If the
same source has any unrelated parse diagnostic, that source remains excluded
from semantic analysis. A resolution-dependent non-generic lowercase candidate
uses the delayed diagnostic boundary described above instead of a parser
diagnostic. A final segment whose initial is neither ASCII uppercase nor ASCII
lowercase, such as `_ready`, reports the parser diagnostic without retaining
refinement metadata and blocks semantic analysis of that source.

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
functions, module immutable values in value positions, source ADT types and
constructors, schemas in schema positions, nominal effects in effect
positions, and handlers in `handle` expressions. The permission is
exact-target and non-transitive: bare names, wrong targets, missing imports,
integration modules, and external packages do not gain it.

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
`Vec<T>`, `Dict<String, T>`, and `Result<Ok, Err>`. Same-package, same-module
source ADTs and public imported source ADTs are eligible when every constructor
payload also has an eligible shape. A qualified imported ADT is selected from
the import target package. A same-spelled ADT in another package cannot supply
its visibility or payload shape. Each selected package identity is checked
independently, so source loading order and the eligibility of a namesake ADT do
not change the result. Both directions also accept singleton and finite-set
refinements of those ADTs recursively through the same containers and through
composed format-neutral schemas. Decode retains each refinement in the helper
parameter and successful result instead of strengthening a base ADT after the
call. Field-predicate name validation uses the same retained field shape.

Only source-declared ADTs are eligible for these refinement fields. A finite
compiler-provided source-less ADT remains usable in an ordinary refinement
annotation, but a refinement such as `Option<Int>::Some` prevents the schema
from publishing a format-neutral encode helper. An ordinary container such as
`Option<Result<Int, String>>` remains eligible when its recursively visited
arguments satisfy the field rules.

A private constructor can appear only in a private schema declared by its own
package and module. The exact test companion may explicitly import that target
and decode or encode through the target's private schema with the private
constructor. A same-spelled module in another package does not gain private
access. A public schema boundary and an ordinary imported schema user cannot
expose a private constructor through a generated helper. Direction-specific
eligibility checks apply to the constructors written in nested singleton and
finite-set refinements before a singleton or complete set is normalized to
another type form. Provider-backed opaque ADTs do not become eligible
refinements. Private imports other than exact-companion access, missing or
wrong-kind targets, and unsupported ADT payloads are declaration errors. Decode
and encode otherwise share this vocabulary but differ in recursive generic
stopping: decode may accept a repeated source ADT descriptor with changed
instantiated arguments; encode checks newly introduced arguments and rejects
unsupported leaves. There is no separate container-depth limit.

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
Full imported paths take precedence over local import qualifiers. An explicit
alias supplies its own qualifier, while an unaliased import supplies its
implicit leaf qualifier. Colliding workspace qualifiers remain unresolved
regardless of import order. Format-neutral schemas may compose only
format-neutral targets; binary schemas only binary targets. Missing, private,
wrong-kind, ambiguous, incompatible, cyclic, duplicate-binding,
forward-reference, and direction-specific helper failures are declaration
errors. The private-target case has one exception: an exact test companion with
an explicit import may compose a private schema from its target module. This
access does not extend to another module or package.

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
are available through written imports. An exact test companion may also use an
explicitly imported private schema from its target module, including a decode
or encode operation whose schema-local record contains a private refinement
admitted by the format-neutral field rules. A written import does not create a
bare schema operation path. Generated helper names are implementation details.

`validate SchemaName from value at base_offset` accepts a format-neutral schema
whose refinement-bearing positions are record fields of that schema, nested
anonymous record fields, or fields of recursively composed format-neutral
schemas. These positions may pass through `Option<T>`, `Result<T, E>`,
`List<T>`, `Vec<T>`, and `Dict<String, T>` in any supported nesting. Each
terminal position contains a singleton or finite-set source-ADT refinement.
These container names and the dictionary key name resolve to the standard
types. A source ADT with the same name is not a validation container or a
string dictionary key.
The value is checked against the schema-local record after recursively
replacing each supported refinement with its base ADT while preserving every
record shape, container identity, and generic argument position, and
`base_offset` must be `ByteOffset`. The result is
`Result<SchemaRecord, DecodeError>` with the refinements retained. A supported
validation schema has at least one reachable such field. It and every reachable
composed schema have no field-local or schema-level predicate. Other supported
fields may contain unrefined containers, which runtime validation preserves
without traversing. Composed branches and anonymous record branches without
refinements are also preserved, but composed branches still participate in the
recursive static eligibility check. A dictionary on a refinement path must use
`String` keys. Refinement erasure inside arbitrary source-ADT payloads or
recursive values is not eligible. Binary schemas are also not eligible. The value and offset
expressions are evaluated once, in that order. Schema-path lookup,
public aliases, and exact-companion private access follow the decode and encode
rules above. This expression use of `validate` is distinct from a schema-body
validation predicate.

The schema path is required. `validate from value at base_offset` reports
`parse.schema_validate_expression` with `schema path` as the expected input and
is not accepted source. Parser recovery retains the validation expression and
its value and offset operands so syntax formatting and later source tooling can
preserve the surrounding structure.

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
productions describe accepted form. The cleanup nesting resource limit is the
prose contract above. The remainder predicate resource limit is the prose
contract below and is enforced by the executable grammar.

The `%` binary operator is accepted in ordinary expressions, function and
test contract predicates, and hole `satisfy` predicates. Schema field and
schema-level validation predicates still reject `%`. The
[operator](types.md#operators-and-diagnostics), [contract](contracts.md), and
[hole](holes.md#satisfy-syntax-and-binding) pages define typing, evaluation, and
repair-analysis behavior for the accepted forms.

When a function or test contract predicate or a hole `satisfy` predicate
contains `%`, at most 128 opening parentheses may be simultaneously unmatched
in that predicate root. The count includes call parentheses and parentheses in
a Boolean branch that does not contain `%`. The next opening parenthesis makes
the source form invalid. This limit does not apply to predicates without `%`,
ordinary expressions, or schema predicates.

A handler declaration requires `for` between its parameter list and its one
nominal effect target. The former `handles` spelling is an ordinary identifier,
so it may be used wherever the corresponding identifier casing rules permit.
Using it as the handler separator is rejected with `for` as the expected token.
When `for` is missing, a following member path remains the recovered effect
target. This includes the bare target `handles` and a qualified target that
starts with `handles::`. If `handles` is instead followed by another identifier
that can start the target, recovery rejects the first `handles` as the former
separator and retains the following member path as the target. Both forms keep
the optional retained-effect list and the boundary of the next declaration.

<!-- source-surface-grammar:start -->
```text
Module        ::= ModuleHeader? UseDecl* Item*
ModuleHeader  ::= "mod" ModuleHeaderPath NL
UseDecl       ::= "use" ModulePath UseAlias? ImportSource? NL
UseAlias      ::= "as" AliasName
AliasName     ::= Identifier
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
Item          ::= ModuleValueDecl | Function | TestDecl | EffectDecl | HandlerDecl | TypeDecl | SchemaDecl | PublicAlias
ModuleValueDecl ::= "pub"? "let" ModuleBindingName ":" TypeText "=" Expr DeclarationEnd
DeclarationEnd ::= NL | EOF
ModuleBindingName ::= ordinary identifier with ASCII lowercase initial
Function      ::= "pub"? "fn" Name EffectBinder? "(" ParamList? ")" Return? Effects? CallsiteModifier? NL
                  Contract* Body "end" NL?
TestDecl      ::= "test" Name "(" ")" Return Effects? NL
                  Contract* Body "end" NL?
TypeDecl      ::= "pub"? "type" Name TypeParamList? NL TypeVariant* "end" NL?
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
SchemaFieldWhere ::= "where" (SchemaPredicate | ByteViewMultiplePredicate)
ByteViewMultiplePredicate ::= "payload_count" "multiple" "of" (Name | IntLiteral)
SchemaValidation ::= "validate" SchemaPredicate NL
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
VariantAlternative ::= NamedAdtBase "::" UpperName | TypePath RefinementTypeArguments "::" UpperName
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
Contract      ::= ("require" | "ensure" | "invariant") FunctionContractPredicate NL
FunctionContractPredicate ::= FunctionContractPredicatePrefix (FunctionContractPredicateOp FunctionContractPredicatePrefix)*
FunctionContractPredicateOp ::= SchemaPredicateOp | "%"
FunctionContractPredicatePrefix ::= ("not" | "-" | "~") FunctionContractPredicatePrefix | FunctionContractPredicatePostfix
FunctionContractPredicatePostfix ::= FunctionContractPredicatePrimary (FunctionContractPredicateCall | FieldAccess)*
FunctionContractPredicatePrimary ::= Literal | NamePath | "perform" MemberPath "::" Name FunctionContractPredicateCall | "(" FunctionContractPredicate ")" | "()"
FunctionContractPredicateCall ::= "(" FunctionContractPredicate ("," FunctionContractPredicate)* ","? ")" | "(" ")"
SatisfyPredicate ::= SatisfyPredicatePrefix (SatisfyPredicateOp SatisfyPredicatePrefix)*
SatisfyPredicateOp ::= SchemaPredicateOp | "%"
SatisfyPredicatePrefix ::= ("not" | "-" | "~") SatisfyPredicatePrefix | SatisfyPredicatePostfix
SatisfyPredicatePostfix ::= SatisfyPredicatePrimary (SatisfyPredicateCall | FieldAccess)*
SatisfyPredicatePrimary ::= Literal | NamePath | "perform" MemberPath "::" Name SatisfyPredicateCall | "(" SatisfyPredicate ")" | "()"
SatisfyPredicateCall ::= "(" SatisfyPredicate ("," SatisfyPredicate)* ","? ")" | "(" ")"
SchemaPredicate ::= SchemaPredicatePrefix (SchemaPredicateOp SchemaPredicatePrefix)*
SchemaPredicateOp ::= "or" | "and" | "|" | "^" | "&" | "==" | "!="
                  | "<" | "<=" | ">" | ">=" | "<<" | ">>" | ">>>"
                  | "+" | "-" | "*" | "/"
SchemaPredicatePrefix ::= ("not" | "-" | "~") SchemaPredicatePrefix | SchemaPredicatePostfix
SchemaPredicatePostfix ::= SchemaPredicatePrimary (SchemaPredicateCall | FieldAccess)*
SchemaPredicatePrimary ::= Literal | NamePath | "perform" MemberPath "::" Name SchemaPredicateCall | "(" SchemaPredicate ")" | "()"
SchemaPredicateCall ::= "(" SchemaPredicate ("," SchemaPredicate)* ","? ")" | "(" ")"
Body          ::= (LetLine | DeferStatement | ExprLine)*
LetLine       ::= "let" LetPattern (":" TypeText)? "=" Expr NL
DeferStatement ::= "defer" NL Body "end" NL?
LetPattern    ::= "_" | BindingName | ConstructorPattern | RecordPattern
ExprLine      ::= Expr NL
Expr          ::= PrefixExpr (BinaryOp PrefixExpr)*
BinaryOp      ::= "|>" | "or" | "and" | "|" | "^" | "&" | "==" | "!="
                  | "<" | "<=" | ">" | ">=" | "<<" | ">>" | ">>>"
                  | "+" | "-" | "*" | "/" | "%"
PrefixExpr    ::= ("not" | "-" | "~") PrefixExpr | PostfixExpr
PostfixExpr   ::= PrimaryExpr (Call | TypeArgs | FieldAccess | "?")*
PrimaryExpr   ::= Hole | Literal | NamePath | Perform | Handle | SchemaDecode | SchemaEncode | SchemaValidate | BeginExpr | "(" Expr ")" | "()"
                  | Record | Dict | List | Match | If
Hole          ::= ("_" | HoleName) SatisfyClause?
SatisfyClause ::= "satisfy" Name "=>" SatisfyPredicate
BeginExpr     ::= "begin" NL Body "end"
SchemaDecode  ::= "decode" MemberPath "from" Expr "at" Expr
SchemaEncode  ::= "encode" MemberPath "from" Expr
SchemaValidate ::= "validate" MemberPath "from" Expr "at" Expr
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
- Formatter and recovered-import preservation:
  `crates/veln-syntax/src/format/declarations.rs` and
  `crates/veln-syntax/src/tests/import_aliases.rs`.
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
