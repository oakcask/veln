# Source Surface

This file specifies the source subset implemented by the parser and AST.

## Grammar

The formal grammar block is generated from the executable Prolog source
surface specification. Keep semantic behavior, diagnostics, and rationale in
the surrounding prose.

<!-- source-surface-grammar:start -->
```text
Module        ::= ModDecl? UseDecl* Item*
ModDecl       ::= "mod" ModuleName NL
UseDecl       ::= "use" ModuleName NL
ModuleName    ::= Name ("." Name)*
Item          ::= Function | TestDecl | TypeDecl
Function      ::= "pub"? "fn" Name "(" ParamList? ")" Return? Effects? NL
                  Contract* Body "end" NL?
TestDecl      ::= "test" Name "(" ")" Return Effects? NL
                  Contract* Body "end" NL?
TypeDecl      ::= "pub"? "type" Name TypeParamList? NL TypeVariant+ "end" NL?
TypeParamList ::= "(" Name ("," Name)* ","? ")"
TypeVariant   ::= "pub"? UpperName TypeVariantFields? NL
TypeVariantFields ::= "(" TypeVariantField ("," TypeVariantField)* ","? ")"
                  | "{" TypeVariantField ("," TypeVariantField)* ","? "}"
TypeVariantField ::= Name ":" TypeText | TypeText
ParamList     ::= Param ("," Param)* ","?
Param         ::= Name (":" TypeText)?
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
                  | Record | Dict | List | Match
Call          ::= "(" ArgList? ")"
ArgList       ::= Expr ("," Expr)* ","?
TypeArgs      ::= "[" TypeText ("," TypeText)* ","? "]"
FieldAccess   ::= "." Name
Record        ::= "{" (Name ":" Expr) ("," Name ":" Expr)* ","? "}"
Dict          ::= "{" Expr ":" Expr ("," Expr ":" Expr)* ","? "}"
List          ::= "[" ArgList? "]"
Match         ::= "match" Expr NL MatchArm+ "end"
MatchArm      ::= Pattern "=>" Expr NL
Pattern       ::= "_" | BindingName | Literal | ConstructorPattern | RecordPattern
ConstructorPattern ::= ConstructorName "(" PatternList? ")" | ConstructorName
ConstructorName ::= UpperName | Name "::" Name ("::" Name)*
RecordPattern ::= "{" PatternFieldList? "}"
PatternList   ::= Pattern ("," Pattern)* ","?
PatternFieldList ::= PatternField ("," PatternField)* ","?
PatternField  ::= Name ":" Pattern
```
<!-- source-surface-grammar:end -->

`Name` is an identifier. `UpperName` is an identifier whose first character is
uppercase. `BindingName` is an unqualified identifier whose first character is
not uppercase. `TypeText` is collected from source and parsed by the semantic
type parser. Contract predicates parse through a narrower predicate production
before semantic contract validation.

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
record expression. Literal and constructor patterns are match-only in the
implemented slice; using one in a `let` statement reports
`pattern.refutable_let`.

A return may name the returned value for postconditions with `-> name: Type`.
The binding is contract-facing only: it is visible to `ensure` clauses for the
same function and to runtime `ensure` checks for tail-expression returns and
`?` early returns, but not to `require` clauses, the function body, or callers.
Bare `result` has no special meaning.

`mod` declares the source module identity. The header is optional for a
single-file source with no imports. A source file with one or more `use`
declarations must declare `mod` before those imports.

When a project root contains `veln.toml`, the implemented manifest subset may
list source modules in a `[modules]` table:

```toml
[modules]
"src/main.veln" = "app.main"
```

The source `mod` declaration remains the compiler-visible owner of the module
name. A manifest entry is packaging/discovery metadata and cannot rename the
source module. If the manifest name differs from the source `mod` name, or if
the manifest names a selected source file that has no `mod` declaration, the
checker reports module metadata drift.

`use` declarations create module import aliases. The current alias is the final
segment of the imported module path, so `use platform.io` declares the alias
`io`. Calls may use that alias as a qualified function path, such as
`io::read_line()`, when the imported module's source is part of the analyzed
program. Public source ADT constructors may also use the import alias, either
as `alias::Constructor` or `alias::Type::Constructor`.

Public `fn` declarations and public source `type` declarations are the
implemented public API boundary. Dedicated export lists are not implemented.
Function declarations can be referenced by bare name or by a `use`
alias-qualified path as callable values where a function-typed expression is
expected. When a selected `run` or `test` entry uses a function declaration as a
value, that referenced function is part of the selected executable slice. In a
named source module, selected-entry reachability treats a bare function
reference as a reference to the same source module. `use` alias-qualified
references keep the imported module identity. Bare local bindings, parameters,
and match-pattern bindings shadow same-named function declarations for this
reachability rule. Calls through a function-typed local binding or parameter
conservatively include visible function declarations with the same argument
count when surface reachability cannot prove one concrete declaration target.

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
  `Result((), <TypePath>)` and append an implicit `Ok(())` success value. If the
  fence omits `error=<TypePath>`, contains `?`, and immediately documents a
  public function with an explicit `Result(_, E)` return type, the generated
  wrapper uses `Result((), E)` and also appends the implicit `Ok(())` success
  value. If there is no documented result context, the wrapper error type is
  inferred when every `?` applies to a known function call returning
  `Result(_, E)` and all such calls use the same `E`.
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
## decision: Keep the source header canonical.
## consequences: Manifest metadata cannot rename the module.
mod app.core
```

The parser exposes complete ADR-lite records as structured source metadata and
attaches each record to the nearest following `mod` declaration or `pub fn`
declaration when one exists. ADR-lite records are ignored for runtime
semantics: they do not affect parsing of declarations, type checking,
lowering, execution, or generated output.

## Expressions

Implemented expressions:

- holes: `_` and `_name`, with optional `satisfy candidate => predicate`
- literals: strings, integers, floats, `true`, `false`, and `()`
- paths and calls: `name`, `module::name`, `callee(args...)`
- type-applied call callees: `callee[TypeText](args...)`
- callable function declaration values by bare name
- constructors: `Ok(value)`, `Err(error)`, `Some(value)`, `None`, `Nil`,
  `Cons(head, tail)`, and their `Result::`, `Option::`, or `List::`
  qualified forms
- channel effect calls: `channel::bounded(capacity)`,
  `channel::bounded[Item](capacity)`, `channel::clone(tx)`,
  `channel::send(tx, value)`, `channel::recv(rx)`,
  `channel::select(left, right)`,
  `channel::select_priority(left, right)`,
  `channel::select_timeout(left, right, timeout_ms)`,
  `channel::select_result(left, right)`,
  `channel::select_priority_result(left, right)`,
  `channel::select_timeout_result(left, right, timeout_ms)`, and
  `channel::close(tx)`
- task effect calls: `task::spawn(job)`, `task::spawn[Item](job)`,
  `task::join(task)`, and `task::cancel(task)`
- prelude helpers as bare calls such as `vec_len(items)`
- records: `{name: value, ...}`
- dictionaries: `{key_expr: value_expr, ...}` when the first entry is not a
  bare `name: value` field; identifier-led expression keys such as `seed + 1`
  are dictionary keys
- record field access: `expr.name`
- vec literals: `[value, ...]`
- match expressions over literals, bindings, `_`, record patterns, and
  descriptor-backed constructors `Some`, `None`, `Ok`, `Err`, `Nil`, `Cons`,
  and their `Option::`, `Result::`, or `List::` qualified forms
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
with generic parameters, nullary variants, tuple-like variants, and
record-shaped variant declarations:

```text
pub type Maybe(A)
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
Private constructors remain usable in their declaring module. The built-in
`List(A)` descriptor recognizes `Nil`, `Cons(head, tail)`, `List::Nil`, and
`List::Cons(head, tail)` and keeps the existing runtime list representation.

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

Type-applied call callees currently contribute static item-type information
only for recognized built-in calls such as `channel::bounded[String](capacity)`.
They are not a general user-defined generic function mechanism.

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
can classify as finite domains: `Bool`, `Option(T)`, `Result(T, E)`, `List(A)`,
and source-declared ADTs. `_` and binding patterns are catch-all arms. Bool
matches must cover `true` and `false`; option matches must cover `Some(_)` and
`None`; result matches must cover `Ok(_)` and `Err(_)`; list matches must cover
`Nil` and `Cons(_)`; source-declared ADT matches must cover every declared
variant unless a catch-all arm is present.

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
program's function names or the prelude helper set, and qualified calls resolve
through `use` aliases. Call arguments must be assignable to declared parameter
types. Function declaration values may be passed to contract calls where the
callee expects a function type; bare references resolve against visible
function declarations, and `use` alias-qualified references keep the imported
module identity. Numeric return values from pure calls may be used inside
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

Implemented lowering and execution do not include user-defined ADT
declarations, method calls, loops, mutation, classes, traits, macros,
comprehensions, anonymous functions, custom operators, task selection, package
manifest fields beyond `[modules]`, foreign declarations, or doctest metadata
other than `error`, `ignore`, `fail`, `runtime=contract`, runtime contract
detail attributes, `runtime=ensure`, runtime ensure detail attributes,
`runtime=result`, runtime result value matching, and `veln-output` stream
selection.
