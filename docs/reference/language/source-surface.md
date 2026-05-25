# Source Surface

This file specifies the source subset implemented by the parser and AST.

## Grammar

```text
Module        ::= ModDecl? UseDecl* Item*
ModDecl       ::= "mod" ModuleName NL
UseDecl       ::= "use" ModuleName NL
Item          ::= Function | TestDecl
Function      ::= "pub"? "fn" Name "(" ParamList? ")" Return? Effects? NL
                  Contract*
                  BodyLine*
                  "end" NL?
TestDecl      ::= "test" Name "(" ")" Return Effects NL
                  Contract*
                  BodyLine*
                  "end" NL?
Param         ::= Name (":" TypeText)?
Return        ::= "->" ResultBinding? TypeText
ResultBinding ::= Name ":"
Effects       ::= "effects" "[" EffectList? "]"
Contract      ::= ("require" | "ensure") ContractPredicate NL
BodyLine      ::= LetLine | ExprLine
LetLine       ::= "let" LetPattern (":" TypeText)? "=" Expr NL
LetPattern    ::= "_" | BindingName | RecordPattern
ExprLine      ::= Expr NL
Record        ::= "{" (Name ":" Expr) ("," Name ":" Expr)* ","? "}"
Dict          ::= "{" Expr ":" Expr ("," Expr ":" Expr)* ","? "}"
Match         ::= "match" Expr NL MatchArm+ "end"
MatchArm      ::= Pattern "=>" Expr NL
TypeArgs      ::= "[" TypeText ("," TypeText)* ","? "]"
Pattern       ::= "_" | BindingName | Literal | ConstructorPattern | RecordPattern
ConstructorPattern ::= ConstructorName "(" PatternList? ")"
                     | ConstructorName
ConstructorName ::= UpperName | Name "::" Name ("::" Name)*
RecordPattern ::= "{" PatternFieldList? "}"
PatternList   ::= Pattern ("," Pattern)* ","?
PatternFieldList ::= PatternField ("," PatternField)* ","?
PatternField  ::= Name ":" Pattern
```

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
only and does not define parse structure.

When a declaration returns a function type that itself carries effects, write
the function-type effect list before the declaration effect list:
`-> fn(String) -> () effects [stdio] effects []`. The first `effects [...]`
belongs to the returned function type; the second belongs to the enclosing
declaration.

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
same function and to runtime `ensure` checks for ordinary returns, but not to
`require` clauses, the function body, or callers. Bare `result` has no special
meaning.

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
program.

Public `fn` declarations are the implemented public API boundary. Dedicated
export lists are not implemented. Function declarations can be referenced by
bare name as callable values where a function-typed expression is expected.
When a selected `run` or `test` entry uses a function declaration as a value,
that referenced function is part of the selected executable slice.

`test` is a top-level declaration keyword, not a visibility modifier. Test
declarations are selected by `veln test` from `*_test.veln` files, explicit
targets, and any automatically discovered source file that contains a top-level
`test` declaration. They require an empty parameter list, require an explicit
return type and `effects [...]` clause, and are not ordinary callable
functions.

Documentation line comments may contain executable doctest fences. A doc
comment fence whose info string is `veln` is extracted as generated test source
for `veln check` and `veln test`. The fence may include `error=<TypePath>` to
make the generated wrapper return `Result((), <TypePath>)` and append an
implicit `Ok(())` success value. If the fence omits `error=<TypePath>`, contains
`?`, and immediately documents a public function with an explicit
`Result(_, E)` return type, the generated wrapper uses `Result((), E)` and also
appends the implicit `Ok(())` success value. If there is no documented result
context, the wrapper error type is inferred when every `?` applies to a known
function call returning `Result(_, E)` and all such calls use the same `E`. Any
other `veln` fence attribute reports `doctest.unknown_metadata`, and an empty
`error=` value reports `doctest.invalid_metadata`. A following doc comment fence
whose info string is `veln-output stream=stdout` or
`veln-output stream=stderr` attaches expected output to the immediately
preceding generated doctest. A `veln ignore` fence is treated as a
documentation-only code example and does not create a generated doctest.
An executable fence marked `veln fail` is a negative static example. It is
checked as a generated private function and is accepted only when that generated
source produces at least one parse or semantic diagnostic. It is not selected
as a runtime doctest case and cannot attach expected output.
Inside an executable `veln` fence, a line that starts with `# ` is hidden setup:
the generated test includes the line after removing the marker. Hidden setup is
useful for imports, helpers, and bindings that the documented sample should use
without displaying harness-only setup as example code.
Unknown `veln-output` attributes, missing `stream`, and stream values other
than `stdout` or `stderr` report doctest metadata diagnostics. A doctest may
attach at most one expected-output fence for each stream. A second
`veln-output` fence for the same stream reports
`doctest.duplicate_output` at the duplicate fence and leaves the first fence as
the selected expectation.

Documentation line comments may also contain ADR-lite records. A complete
record starts with `/// @adr` or `/// @adr-lite` and then provides these fields
as `key: value` doc-comment lines: `id`, `status`, `scope`, `context`,
`decision`, and `consequences`.

```veln
/// @adr
/// id: module-boundary
/// status: accepted
/// scope: module
/// context: Module identity is compiler-visible.
/// decision: Keep the source header canonical.
/// consequences: Manifest metadata cannot rename the module.
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
- constructors: `Ok(value)`, `Err(error)`, `Some(value)`, `None`, and their
  `Result::` or `Option::` qualified forms
- channel effect calls: `channel::bounded(capacity)`,
  `channel::bounded[Item](capacity)`, `channel::clone(tx)`,
  `channel::send(tx, value)`, `channel::recv(rx)`, and `channel::close(tx)`
- prelude helpers as bare calls such as `list_len(items)`
- records: `{name: value, ...}`
- dictionaries: `{key_expr: value_expr, ...}` when the first entry is not a
  bare `name: value` field; identifier-led expression keys such as `seed + 1`
  are dictionary keys
- record field access: `expr.name`
- lists: `[value, ...]`
- match expressions over literals, bindings, `_`, record patterns, and built-in
  constructors `Some`, `None`, `Ok`, `Err`, `Option::Some`, `Option::None`,
  `Result::Ok`, and `Result::Err`
- prefix operators: `not`, `-`
- pipelines: `expr |> target(args...)`
- binary operators: `or`, `and`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `+`, `-`,
  `*`, `/`
- postfix result propagation: `expr?`
- parenthesized expressions

`true` and `false` are boolean literals in expression and pattern positions;
they are not ordinary value names.

A `satisfy` suffix is valid only on a hole expression. The suffix requires one
candidate binding, the `=>` separator, and a predicate. The candidate binding
is visible only inside the suffix predicate.

Pipelines require a named or qualified call expression on the right. The piped
expression is inserted as the first argument of that call, so
`value |> target(extra)` is checked and executed as `target(value, extra)`. A
non-call target, or a call whose callee is not a name path, reports
`type.pipeline_target`.

Type-applied call callees currently contribute static item-type information
only for recognized built-in calls such as `channel::bounded[String](capacity)`.
They are not a general user-defined generic function mechanism.

Method-call-shaped syntax, such as `value.field(args)`, is rejected during
parsing with `parse.method_call`. Use a plain function call like
`field(value, args)` and reserve `value.field` for record field access.
Call arguments must be separated with commas and closed with `)`. When the
parser can identify an adjacent argument without a separator, it reports
`parse.call_argument` and continues as if a comma had been inserted.

`match` is a primary expression and may appear anywhere an expression is
accepted, including call arguments and aggregate literals. Match arms are tried
in source order. The implemented match-pattern subset covers wildcard `_`,
binding names, literals, record patterns, and the built-in constructors `Some`,
`None`, `Ok`, `Err`, `Option::Some`, `Option::None`, `Result::Ok`, and
`Result::Err`. Record patterns match when the scrutinee is a record containing
every named pattern field and every nested field pattern matches. Pattern
bindings in one arm or `let` statement must not duplicate another binding in
that pattern or a value binding already visible at the pattern. Record pattern
field names must be unique. Exhaustiveness is not statically checked in the
current slice.

## Contract Predicates

`require`, `ensure`, and hole `satisfy` predicates accept this implemented
syntax:

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
pure functions, call arguments must be assignable to declared parameter types,
and field access must resolve through record-typed values visible to the
clause.

Valid clauses are executable obligations. `require` is checked at function
entry. `ensure` is checked before an ordinary tail-expression return and may
read an explicit result binding.

## Not Implemented

Implemented lowering and execution do not include user-defined ADT
declarations, method calls, loops, mutation, classes, traits, macros,
comprehensions, anonymous functions, custom operators, channel `spawn`, task
handles, cancellation, join, or selection, package manifest fields beyond `[modules]`, foreign
declarations, or doctest metadata other than `error`, `ignore`, `fail`, and
`veln-output` stream selection.
