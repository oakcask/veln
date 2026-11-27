---
role: proposal
update-when: Named ADT variant declaration, construction, pattern, evaluation, formatting, tooling, or standard-library migration decisions and their implementation acceptance change.
---

# Named ADT Variant Fields

Add named construction and destructuring for variants whose declaration uses
braces. Callers can identify payload roles without counting positional
arguments, especially when several fields have the same type.

This is proposed behavior. All acceptance evidence below is planned, not
implemented by this document.

```veln
type Foo
	Bar { name: String, id: Int }
	Empty
end

let value = Bar { name: "Alice", id: 42 }

match value
	Bar { name: name, id: _ } => name
	Empty => ""
end
```

## Current boundary

The current [source surface](../specification/source-surface.md) already
accepts both braces and parentheses in variant declarations, with named or
positional payload entries. That declaration syntax alone does not provide
the construction and pattern forms proposed here. Constructor patterns in the
current grammar use parentheses or a bare constructor name.

The syntax parser retains the declaration delimiter, but AST lowering does
not retain it as a variant-level distinction. Individual fields retain their
names and positional markers. The implementation must preserve the new
variant-level contract across lowering and downstream consumers; recognizing
the example declaration alone is insufficient.

Inspection basis: `crates/veln-syntax/src/parser/schemas.rs`,
`crates/veln-syntax/src/parser/expression_patterns.rs`,
`crates/veln-syntax/src/format/declarations.rs`,
`crates/veln-ast/src/model.rs`, and `crates/veln-ast/src/lower.rs`.
The current [type rules](../specification/types.md) remain the authority for
constructor inference and ADT identity until implementation changes them.

## Variant forms

| Declaration | Construction | Destructuring | Meaning |
| --- | --- | --- | --- |
| `Empty` | `Empty` | `Empty` | Retained nullary form. |
| `Pair(String, Int)` | `Pair("Alice", 42)` | `Pair(name, id)` | Retained positional form. |
| `Pair(name: String, id: Int)` | `Pair("Alice", 42)` | `Pair(name, id)` | Labels in a parenthesized declaration do not enable named construction. |
| `Bar { name: String, id: Int }` | `Bar { name: "Alice", id: 42 }` | `Bar { name: name, id: id }` | New named-field contract. |

Variants within one ADT may use different forms. A brace declaration must
contain at least one explicitly named field. Duplicate names, unnamed entries,
and mixed named/unnamed entries in braces are invalid. Empty brace declarations
and empty brace construction/patterns are invalid; use a nullary variant.
Parenthesized declarations retain their existing payload-entry rules.

A named variant accepts only named construction and named constructor
patterns. `Bar("Alice", 42)`, `Bar({name: "Alice", id: 42})`, and
`Bar(name, id)` as a pattern are invalid for that declaration. Conversely,
`Pair { name: "Alice", id: 42 }` is invalid for a positional declaration.
Anonymous records remain structural values; a named variant remains part of
its nominal ADT and is not assignable to a record of the same field shape.

## Syntax and boundaries

The following grammar describes the new forms. Existing name, type,
expression, and pattern productions retain their meanings.

```ebnf
NamedVariantDecl ::= "pub"? UpperName NamedVariantFields NL
NamedVariantFields ::= "{" NamedVariantField ("," NamedVariantField)* ","? "}"
NamedVariantField ::= Name ":" TypeText
NamedConstructorExpr ::= ConstructorName TypeArgs? NamedInitializer
NamedInitializer ::= "{" NamedArgument ("," NamedArgument)* ","? "}"
NamedArgument ::= Name ":" Expr
NamedConstructorPattern ::= ConstructorName "{" NamedPatternField
                            ("," NamedPatternField)* ","? "}"
NamedPatternField ::= Name ":" Pattern
```

Constructor names use the existing unqualified, ADT-qualified, imported, and
alias-qualified resolution rules. Explicit expression type arguments, when
written, use the existing constructor type-argument order and arity:
`Boxed<Int> { value: 1 }`. Patterns retain their existing type-inference
mechanism and do not gain explicit type arguments.

The opening brace must follow the constructor path, or its explicit type
arguments, without an intervening newline. The same boundary applies between
a declaration's variant name and its opening brace. A comment ending that
line does not continue the form. Within braces, newlines and comments are
allowed between complete list entries and punctuation, including after the
opening brace and before the closing brace. Commas separate entries; a newline
alone does not replace a comma or split a nested expression.

Construction is a single expression with a token-delimited field list, not a
statement body. Braces introduce neither a lexical scope nor a cleanup scope.
An initializer accepts one expression, including an explicitly delimited
`begin`, `if`, or `match`. A bare statement sequence is invalid.
Postfix operations apply to the complete constructed value under existing
typing rules. A brace initializer is restricted to a constructor path;
`make_variant() { name: "Alice" }` is invalid.

Named constructors can appear directly in a match scrutinee. The closing
brace ends construction, and the newline starts the arm list. Unlike languages
with brace-delimited match bodies, no additional parentheses are required:

```veln
match Bar { name: "Alice", id: 42 }
	Bar { id: _, name: name } => name
end
```

Removing leading indentation from these examples preserves ownership and
nesting. Matching delimiters and meaningful newlines determine the boundaries.

## Construction, typing, and evaluation

Each declared field must occur exactly once. Unknown, duplicate, missing, and
wrongly typed fields are errors. Field order is unrestricted and associates
values by name. Field names are labels, not variable references.

Existing constructor inference applies after associating each initializer
with its declared field type. Expected ADT type arguments flow into field
expressions, and fields contribute constraints to generic inference. Generic
ambiguity, visibility, alias identity, variant refinements, variance, and
publication checks retain their current rules. A named variant's field labels
have the visibility of that variant; this proposal adds no field-level
visibility modifier.

Field expressions evaluate once in written order. With
`Bar { id: next_id(), name: next_name() }`, the trace is `next_id`, then
`next_name`, regardless of declaration order. Evaluation stops at an abrupt
exit. If the first initializer fails or propagates a result, later initializers
do not run and no completed ADT value is published. Earlier side effects are
not rolled back. Existing cleanup and failure rules govern any explicit
`begin` inside an initializer and the enclosing region.

A bare named constructor is not a function value. `let build = Bar` is
invalid; use a function that explicitly constructs the variant. This proposal
does not extend ordinary function calls with named arguments.

## Patterns and bindings

Named constructor patterns associate subpatterns with fields by name and may
reorder them. Every field must be present exactly once. Use `_` for an ignored
field. Unknown, duplicate, and omitted fields are errors. Shorthand such as
`Bar { name, id }` and rest patterns such as `Bar { name: name, .. }` are
outside this proposal.

Subpatterns may contain existing record, literal, wildcard, or constructor
patterns, including nested named constructors. Binding names differ from
labels: `Bar { name: label, id: _ }` binds `label`, not `name`. Existing
duplicate-binding, arm-local scope, `let` pattern admission, refutability,
exhaustiveness, and variant-narrowing rules apply to the corresponding payload
patterns. Reordering named fields does not change matching or coverage.

This proposal adds no direct ADT field-access or record-update rules. Consumers
inspect payloads through patterns. Defaults, field punning, spread/update
syntax, and automatic migration actions are separate work.

## Canonical source and migration

`veln fmt` preserves the selected variant form, field order, and explicit
label/subpattern pairs. It never sorts initializers into declaration order,
changes braces to parentheses, or inserts/removes an explicit `begin`.

A list whose children format onto one line and which contains no comments
formats onto one line. Source newlines alone do not prevent convergence.
Use one space before an opening brace, one space inside each brace, one space
after each colon and comma, and no trailing comma on a single line. This rule
applies to one-field and multi-field declarations, expressions, and patterns;
nested single-line forms remain single-line. Line length alone does not select
a different layout for these new lists.

```veln
# Input
Bar{
 name:String,
 id:Int,
}
let value=Bar{id:42,name:"Alice",}

# Canonical output
Bar { name: String, id: Int }
let value = Bar { id: 42, name: "Alice" }
```

A comment or a child that requires multiple lines selects a multiline list.
Each field starts on its own line, indented one tab beyond the enclosing
construct. The closing brace aligns with that construct's indentation. Every
field has a trailing comma. Comments before a field stay with that field;
same-line trailing comments stay with the preceding field or delimiter;
comments immediately before the closing brace remain there. The formatter
does not move a comment across a field or explicit scope boundary.

```veln
let value = Bar {
	id: 42, # allocated identity
	name: begin
		let label = "Alice"
		label
	end,
}

match value
	Bar {
		name: label, # displayed name
		id: _,
	} => label
end
```

Equivalent whitespace and optional trailing commas converge subject to those
comment and multiline-child conditions. Formatting twice produces identical
output. If malformed input prevents preserving boundaries or comments, report
the formatting failure and leave the source unchanged rather than applying a
partial conversion.

Existing parenthesized declarations, construction, and patterns keep their
form. Existing brace declarations with explicit names acquire the named-only
contract; their positional consumers require a manual semantic migration.
Brace declarations with unnamed entries must be changed manually to explicit
names or parentheses. Formatting does not infer intent, rename labels, or
rewrite call sites. Backward compatibility is not required, but the language
implementation must update affected repository consumers in the same delivery.

## Required standard-library adoption

Language support alone does not complete this proposal. Implementation must
audit all ADT declarations under `crates/veln-stdlib/veln/`, including private
implementation types and distributed test support, and migrate variants whose
long positional payloads obscure field roles to named fields. The audit must
also consider shorter payloads with repeated types or easily confused roles;
an arity threshold alone is not the design criterion.

The following inspected examples establish a required minimum migration scope.
They are not a complete inventory of the library:

| Source module | Variants to migrate | Reading problem |
| --- | --- | --- |
| `crates/veln-stdlib/veln/io.veln` | `Error` | Operation, paths, network context, commitment, and message occupy seven positional slots. |
| `crates/veln-stdlib/veln/fs.veln` | `ReadFile`, `WriteFile`, `UpdateFile`, `AppendFile` | Multiple resource capabilities have distinct roles that positional calls conceal. |
| `crates/veln-stdlib/veln/observe.veln` | `SpanStarted`, `ExportSpan`, `SpanStart`, `SpanFinish`, `LogRecord`, `EventRecord`, `CounterIncrementRecord`, `HistogramSampleRecord`, `ExporterState`, `BufferedExporterState`, `JsonLinesExporter`, `BufferedJsonLinesExporter` | Repeated identities, counts, context, and state become difficult to distinguish by position. |
| `crates/veln-stdlib/veln/http2/core/connection_state.veln` | `CoreConnectionState` | Thirteen payload fields combine independent protocol state components. |
| `crates/veln-stdlib/veln/http2/core/outbound_headers.veln` | Both `Accepted` variants and all `CoreOutboundHeadersFailure` variants | State, stream identifiers, lengths, flags, and diagnostic provenance need explicit roles. |

Resolve same-spelled variants by their owning type. Retain established explicit
field names. Give any currently unnamed payload a semantic name, such as
`state` for a returned connection state, rather than publishing generated
positional labels. Preserve payload meaning, runtime outcomes, capability
authority, effects, and failure/progress guarantees throughout the migration.

Update each migrated declaration, construction, and destructuring site across
the library, companion tests, examples, documentation, compiler-known
descriptors, and backend/host bridges that depend on that representation.
Do not keep positional compatibility constructors solely to avoid migration.
Retained positional variants must have a clear compact payload meaning, such
as the value in `Some(T)` or the head/tail structure of `Cons(T, List<T>)`.
The implementation review must account for additional large variants found
by the audit and explain any retained large positional payload by consumer
semantics. Retire the temporary migration inventory when that review ends.

Completion also requires updating the
[standard-library API design policy](../reference/standard-library-api-design.md)
with a durable rule: use named fields when callers otherwise have to count
positions to distinguish payload roles; review both construction and matching,
especially repeated types, state aggregates, and progress/error metadata.
The policy must retain compact positional forms with clear semantics and add
this decision to its review evidence and revisit triggers. A guideline update
without actual library adoption, or adoption without that update, is incomplete.
These policy and API changes belong to implementation, not this proposal-only PR.

Use the policy's required `$stdlib-api-design` workflow and the
[test placement policy](../reference/toolchain-test-harness.md#test-placement-policy)
for the migration and its behavioral evidence.

## Acceptance and verification

Each row is a completion requirement. Use independent expected values and
traces, rather than treating a formatted or lowered copy as its own oracle.

| Case | Required observation | Planned evidence |
| --- | --- | --- |
| Forms and boundaries | Accept mixed variant forms, nested named syntax, trailing commas, comments, and multiline children; reject missing commas, newline-separated constructor/brace, unnamed brace fields, and empty brace forms. Removing leading indentation preserves parsing. | Accepted/rejected source-surface fixtures and parser tests. |
| Name association | Reversed initializer order still yields the declared `name` and `id`; wrong kind, unknown, missing, duplicate, and ill-typed fields fail. | Semantic cases and executable check/run examples. |
| Generic and nominal behavior | Expected types reach empty generic field values; nested generic constraints and aliases retain identity; ambiguous arguments fail; equivalent anonymous records remain incompatible. | Semantic inference and import/alias cases. |
| Patterns | Reordered, nested, wildcard, and renamed bindings yield expected values; invalid labels, duplicate bindings, and omitted fields fail; existing `let` admission, exhaustiveness, and refinement behavior are preserved. | Semantic pattern cases and executable match/let examples. |
| Evaluation and cleanup | Initializers run once in written order; a failing or propagating first field suppresses later fields; nested explicit cleanup runs under existing region rules. | Runtime trace cases using separate scoped Veln fake handlers for controlled effects. |
| Diagnostics | A primary message identifies the failed field or form at its span; declaration context and repairs appear in related notes; human output retains that context. | Human and JSON check cases for unknown/duplicate/missing fields and wrong constructor form. |
| Formatting | Before/after pairs converge for one, multiple, nested, and multiline fields; comments and `begin` boundaries survive; a second format is identical; field evaluation order is unchanged. | Formatter fixtures, checked CLI cases, and semantic before/after trace checks. |
| Artifacts and tools | Variant form and field identities survive syntax/AST wire round trips, checked artifacts, and backend lowering. Generated docs show braces; hover/navigation resolve the owning constructor and field declaration; imported named variants remain usable. | AST/artifact tests, JVM execution, documentation and LSP/MCP cases for constructor/field navigation. |
| Library migration | Required families and additional audit findings use named declarations and consumers; retained positional choices have reviewed semantic reasons; no stale positional call or pattern remains for a migrated variant. | Source audit, package/export checks, and the affected stdlib companion tests. |
| Library behavior | Migrated errors, resource capabilities, observation payloads, and protocol state retain expected values, progress, authority, and failure behavior. | Behavioral tests in `crates/veln-stdlib/veln/` companions; host/backend cases only for separately identified integration contracts. |
| Durable guidance | The API design policy explains selection, examples, review evidence, and revisit conditions for named versus positional variants. | Semantic documentation review against the completed library audit. |

Implementation completion requires every row, current source/type/format/API
specification updates, and checked executable examples. Language delivery may
be staged across PRs, but the proposal remains open until stdlib adoption and
its guideline update are complete. Remove this proposal and its catalog entry
only then, following the
[proposal lifecycle](../reference/proposal-authoring.md#readiness-and-completion).

## Alternatives and related work

- Keep positional construction with labels only on declarations. This does not
  expose field roles at the call site, so it does not solve the motivating
  readability problem.
- Pass one anonymous record: `Bar({name: "Alice", id: 42})`. This is a useful
  distinct payload design, but introduces an extra record layer and nested
  destructuring. The proposal gives the variant its own named fields instead.
- Add named arguments to every function call. That changes ordinary callable
  types and call matching beyond the ADT use case; keep it out of scope.
- The Rust Reference's [Enumerations](https://doc.rust-lang.org/reference/items/enumerations.html#items.enum.constructor)
  distinguishes tuple variants from variants with named fields and shows brace
  construction. This supports the chosen declaration/use symmetry as a design
  comparison, not as authority for Veln typing or runtime representation.
- Its [Struct expressions](https://doc.rust-lang.org/reference/expressions/struct-expr.html#expr.struct.field)
  permits named fields in any order and separately defines shorthand and update
  syntax. Adopt explicit labels and order-independent field association here;
  leave shorthand and updates out. Rust's brace restrictions in match/if heads
  do not transfer to Veln's newline and `end` boundaries.
- Its [Struct patterns](https://doc.rust-lang.org/reference/patterns.html#patterns.struct)
  demonstrates named enum subpatterns, reordered fields, and optional rest
  patterns. Adapt explicit named subpatterns, while requiring every field and
  `_` for ignored payloads to keep the initial contract bounded. Veln's existing
  binding and refinement rules, rather than Rust's ownership rules, govern them.

The official reference sections were inspected directly. A Veln PR-title search
for variant/field work found no focused prior design to cite; this is not a
claim of novelty. Current repository code and specifications, rather than
unrelated historical PRs, establish the implementation boundary above.
