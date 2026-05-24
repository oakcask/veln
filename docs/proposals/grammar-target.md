# Veln First-Slice Grammar Target

Status: accepted-proposal
Implementation: partially implemented

This is the accepted source grammar target for the first Veln slice. It
consolidates the grammar sketch and later grammar updates from the design-wall
decisions. Historical rationale remains in the linked discussion results.

This document describes the language target. It is not a claim that every
production is implemented in the current parser, AST, lowering, or backend.
Record patterns, wildcard and record `let` patterns, `match` expressions in
nested expression positions, and qualified built-in constructor expressions and
patterns from this target are implemented in the current workspace. Literal and
constructor patterns in `let` remain match-only and report a diagnostic when
used as refutable let patterns.

For the fixed reference of behavior implemented in the current workspace, read
[../reference/README.md](../reference/README.md).

## Notation

- `?` means optional.
- `*` means zero or more repetitions.
- `+` means one or more repetitions.
- Literal tokens are written in double quotes.
- `NL` is a newline separator outside grouping forms.

## Module And Items

```text
Module        ::= ModDecl? UseDecl* Item*
ModDecl       ::= "mod" ModuleName NL
UseDecl       ::= "use" ModuleName NL
Item          ::= FunctionDecl | TestDecl

ModuleName    ::= Name ("." Name)*
QualName      ::= Name ("::" Name)*
```

## Declarations

```text
FunctionDecl  ::= Visibility? "fn" Name Params Return? EffectDecl? NL
                  ContractClause*
                  Block
                  "end" NL?
Visibility    ::= "pub"

TestDecl      ::= "test" Name TestParams Return EffectDecl NL
                  ContractClause*
                  Block
                  "end" NL?
TestParams    ::= "(" ")"

Params        ::= "(" ParamList? ")"
ParamList     ::= Param ("," Param)* ","?
Param         ::= Name (":" Type)?
Return        ::= "->" (Name ":")? Type
EffectDecl    ::= "effects" "[" EffectList? "]"
EffectList    ::= EffectName ("," EffectName)* ","?

ContractClause ::= ("require" | "ensure") ContractPredicate NL
```

Public functions must provide parameter types, a return type, and an explicit
`effects [...]` declaration. Private functions may omit annotations only where
inference can produce complete types and effects. Tests always use empty
parameters, an explicit return type, and an explicit effect declaration.

## Blocks And Statements

```text
Block         ::= Stmt* Expr? NL?
Stmt          ::= LetStmt NL
LetStmt       ::= "let" Pattern (":" Type)? "=" Expr
```

Function and test bodies are newline-separated `let` statements followed by an
optional tail expression. The tail expression is the returned value.

## Expressions

```text
Expr          ::= Pipeline
Pipeline      ::= OrExpr ("|>" PipeTarget)*
PipeTarget    ::= Call
OrExpr        ::= AndExpr ("or" AndExpr)*
AndExpr       ::= Equality ("and" Equality)*
Equality      ::= Compare (("==" | "!=") Compare)*
Compare       ::= Add (("<" | "<=" | ">" | ">=") Add)*
Add           ::= Mul (("+" | "-") Mul)*
Mul           ::= Prefix (("*" | "/") Prefix)*
Prefix        ::= ("not" | "-") Prefix | Postfix
Postfix       ::= Primary ("?" | "." Name)*
Primary       ::= Literal
                | Name
                | Hole
                | Call
                | Record
                | List
                | Match
                | "(" Expr ")"
Call          ::= QualName "(" ArgList? ")"
ArgList       ::= Expr ("," Expr)* ","?
```

General calls use `name(args)` or `module::name(args)`. Method-call-shaped
syntax is not a call form in the first slice; `value.field` is field access.
Pipelines insert the piped expression as the first argument of the target call.

## Aggregate Literals

```text
Record        ::= "{" FieldList? "}"
FieldList     ::= Field ("," Field)* ","?
Field         ::= Name ":" Expr
Dict          ::= "{" DictEntry ("," DictEntry)* ","? "}"
DictEntry     ::= Expr ":" Expr
List          ::= "[" ArgList? "]"
```

Record expressions require explicit `name: value` fields. Shorthand fields,
spreads and update syntax are outside the first slice. In expression position,
`{}` and a first bare `name: value` entry remain record syntax. Other
`key: value` brace entries are dictionary entries, including identifier-led
key expressions such as `seed + 1`.

## Match And Patterns

```text
Match         ::= "match" Expr NL MatchArm+ "end"
MatchArm      ::= Pattern "=>" Expr NL
Pattern       ::= "_"
                | BindingName
                | Literal
                | ConstructorPattern
                | "{" PatternFieldList? "}"
ConstructorPattern ::= ConstructorName "(" PatternList? ")"
                     | ConstructorName
ConstructorName ::= UpperName | QualifiedName
QualifiedName   ::= Name "::" Name ("::" Name)*
BindingName     ::= LowerName
PatternList    ::= Pattern ("," Pattern)* ","?
PatternFieldList ::= PatternField ("," PatternField)* ","?
PatternField  ::= Name ":" Pattern
```

The first slice resolves only the built-in `Option` and `Result` constructors:
`Some(value)`, `Option::Some(value)`, `None`, `Option::None`, `Ok(value)`,
`Result::Ok(value)`, `Err(error)`, and `Result::Err(error)`. User-defined
constructor declarations remain outside the first slice.

## Holes

```text
Hole         ::= HoleAtom HoleSatisfy?
HoleAtom     ::= "_" | "_" Name
HoleSatisfy  ::= "satisfy" BindingName "=>" ContractPredicate
```

The optional `satisfy` suffix is valid only on hole expressions. The candidate
binding is scoped only to the predicate after `=>`; the named-hole label, such
as `_port`, remains a diagnostic and repair label, not a binding.

## Types

```text
Type          ::= UnitType
                | TypePath
                | TypePath "(" TypeList? ")"
                | "{" TypeFieldList? "}"
                | FunctionType
UnitType      ::= "()"
TypePath      ::= QualName
FunctionType  ::= "fn" "(" TypeList? ")" "->" Type FunctionTypeEffect?
FunctionTypeEffect ::= EffectDecl
TypeList      ::= Type ("," Type)* ","?
TypeFieldList ::= TypeField ("," TypeField)* ","?
TypeField     ::= Name ":" Type
```

Type syntax uses type paths and type-argument application rather than a
hard-coded grammar production for every built-in type. Function types may carry
an `effects [...]` suffix.

## Contract Predicates

```text
ContractPredicate ::= ContractOr
ContractOr     ::= ContractAnd ("or" ContractAnd)*
ContractAnd    ::= ContractEquality ("and" ContractEquality)*
ContractEquality ::= ContractCompare (("==" | "!=") ContractCompare)*
ContractCompare ::= ContractAdd (("<" | "<=" | ">" | ">=") ContractAdd)*
ContractAdd    ::= ContractMul (("+" | "-") ContractMul)*
ContractMul    ::= ContractPrefix (("*" | "/") ContractPrefix)*
ContractPrefix ::= ("not" | "-") ContractPrefix | ContractPostfix
ContractPostfix ::= ContractPrimary ("." Name)*
ContractPrimary ::= Literal
                  | Name
                  | ContractCall
                  | "(" ContractPredicate ")"
ContractCall    ::= QualName "(" ContractArgList? ")"
ContractArgList ::= ContractPredicate ("," ContractPredicate)* ","?
```

Contracts use ordinary expression spelling where allowed, but parse through a
narrow predicate production. Holes, `?`, pipelines, `match`, records, lists,
and effect-oriented runtime constructs are not contract predicates in the
first slice.

## Lexical Categories

```text
Literal       ::= StringLiteral | IntLiteral | FloatLiteral | BoolLiteral | UnitLiteral
UnitLiteral   ::= "()"
BoolLiteral   ::= "true" | "false"
Name          ::= Ident
UpperName     ::= IdentStartingWithUppercase
LowerName     ::= IdentNotStartingWithUppercase
EffectName    ::= Name
```

`UpperName` is a name that starts with an uppercase letter. `LowerName` is a
name that does not start with an uppercase letter. A qualified name in pattern
position is treated as a constructor name regardless of the final segment's
case.

The lexer treats newlines outside grouping forms as separators. Indentation is
not parse structure; `veln fmt` owns indentation. Grouping forms are
parentheses, brackets, braces, function declarations before their closing
`end`, test declarations before their closing `end`, and `match` expressions
before their closing `end`.

## Excluded From The First Slice

The first slice excludes statement braces, semicolon-separated statement lists,
indentation-sensitive nesting, method calls, user-defined ADT declarations,
loops, mutation, classes, traits, macros, comprehensions, anonymous functions,
custom operators, package manifests, foreign declarations, and doctest fences.

## Source Decisions

- [First-Slice Grammar](agent-language-spec-wall/result-first-slice-grammar.md)
- [Test Declaration Syntax](../reference/source-decisions/result-test-declaration-syntax.md)
- [Hole Satisfy Source Syntax](../reference/source-decisions/result-hole-satisfy-source-syntax.md)
- [Contract Predicate Parsing](../reference/source-decisions/result-contract-predicate-parsing.md)
- [Public Function Type Boundaries](../reference/source-decisions/result-public-function-type-boundaries.md)
- [Effect Declaration Boundary](../reference/source-decisions/result-effect-declaration-boundary.md)
