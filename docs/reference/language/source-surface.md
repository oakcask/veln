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
LetLine       ::= "let" LetTarget (":" TypeText)? "=" Expr NL
LetTarget     ::= Name | "_"
ExprLine      ::= Expr NL
Record        ::= "{" (Name ":" Expr) ("," Name ":" Expr)* ","? "}"
Dict          ::= "{" Expr ":" Expr ("," Expr ":" Expr)* ","? "}"
Match         ::= "match" Expr NL MatchArm+ "end"
MatchArm      ::= Pattern "=>" Expr NL
Pattern       ::= "_" | BindingName | Literal | ConstructorPattern | RecordPattern
ConstructorPattern ::= ConstructorName "(" PatternList? ")"
                     | ConstructorName
RecordPattern ::= "{" PatternFieldList? "}"
PatternList   ::= Pattern ("," Pattern)* ","?
PatternFieldList ::= PatternField ("," PatternField)* ","?
PatternField  ::= Name ":" Pattern
```

`TypeText` is collected from source and parsed by the semantic type parser.
Contract predicates parse through a narrower predicate production before
semantic contract validation.

In expression position, `{}` and brace literals whose first entry is a bare
`name: value` field parse as records. Other brace literals with `key: value`
entries parse as dictionaries, including keys that are identifier-led
expressions such as `seed + 1`.

Function and test declarations can contain multiple body lines between their
header and closing `end`. Expression newlines end the current body line except
inside grouping forms. Parentheses, brackets, braces, and `match` expressions
keep their inner newlines within the same expression; indentation is formatting
only and does not define parse structure.

`let _ = expr` evaluates the expression and discards the resulting value. It
does not introduce a local binding, and later expressions cannot reference the
discard target. A type annotation on the wildcard target still checks the
right-hand expression against that type.

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
`io`.

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
for `veln check` and `veln test`. A following doc comment fence whose info
string is `veln-output stream=stdout` or `veln-output stream=stderr` attaches
expected output to the immediately preceding doctest.

## Expressions

Implemented expressions:

- holes: `_` and `_name`, with optional `satisfy candidate => predicate`
- literals: strings, integers, floats, `true`, `false`, and `()`
- paths and calls: `name`, `module::name`, `callee(args...)`
- callable function declaration values by bare name
- constructors: `Ok(value)`, `Err(error)`, `Some(value)`, and `None`
- prelude helpers as bare calls such as `list_len(items)`
- records: `{name: value, ...}`
- dictionaries: `{key_expr: value_expr, ...}` when the first entry is not a
  bare `name: value` field; identifier-led expression keys such as `seed + 1`
  are dictionary keys
- record field access: `expr.name`
- lists: `[value, ...]`
- match expressions over literals, bindings, `_`, record patterns, and built-in
  constructors `Some`, `None`, `Ok`, and `Err`
- prefix operators: `not`, `-`
- pipelines: `expr |> target(args...)`
- binary operators: `or`, `and`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `+`, `-`,
  `*`, `/`
- postfix result propagation: `expr?`
- parenthesized expressions

A `satisfy` suffix is valid only on a hole expression. The suffix requires one
candidate binding, the `=>` separator, and a predicate. The candidate binding
is visible only inside the suffix predicate.

Pipelines require a call expression on the right. The piped expression is
inserted as the first argument of that call, so `value |> target(extra)` is
checked and executed as `target(value, extra)`. A non-call pipeline target
reports `type.pipeline_target`.

Method-call-shaped syntax, such as `value.field(args)`, is rejected during
parsing with `parse.method_call`. Use a plain function call like
`field(value, args)` and reserve `value.field` for record field access.

`match` arms are tried in source order. The implemented match-pattern subset
covers wildcard `_`, binding names, literals, record patterns, and the built-in
constructors `Some`, `None`, `Ok`, and `Err`. Record patterns match when the
scrutinee is a record containing every named pattern field and every nested
field pattern matches. Pattern bindings in one arm must not duplicate another
binding in that arm or a value binding already visible at the arm. Record
pattern field names must be unique. Exhaustiveness is not statically checked in
the current slice.

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
comprehensions, anonymous functions, custom operators, package manifest fields
beyond `[modules]`, foreign declarations, doctest result propagation, doctest
error-type fence metadata, or non-output doctest metadata.
