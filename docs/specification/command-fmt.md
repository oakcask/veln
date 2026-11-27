---
role: specification
authority: normative
update-when: The veln fmt command parsing gate, formatting behavior, source discovery behavior, write policy, or executable formatting evidence changes.
specification-coverage: usage=#format-command; behavior=#formatting-rules; limits=#limits-and-errors
---

# Format Command

Use `veln fmt [PATH ...]` to format selected source files. Selection follows
`check`. The command parses every selected file before writing any file. One
parse diagnostic makes the invocation fail and leaves every file unchanged.

## Formatting rules

The formatter deterministically formats imports, function signatures, contract
clauses, typed module `let` declarations, local `let` statements, tail
expressions, holes with `satisfy`, records, lists, calls, literals, paths,
prefix and binary operators, postfix `?`, and supported binary-schema primitive
compatibility spellings. Indentation is one tab per level: top-level items and
closing `end` lines use level zero, and function body lines use level one.

For an explicit import alias, the formatter writes `as alias` after the target
module path and before an optional `from "package"` source. Reformatting the
accepted declaration preserves that order and is idempotent. The
[source-surface specification](source-surface.md#usage-and-declaration-forms)
defines the accepted import syntax and malformed-clause boundary.

A module `let` declaration remains at indentation zero. The formatter writes
the declaration as `[pub] let name: Type = initializer`, applies the ordinary
type layout to its required annotation, and formats its initializer with the
ordinary expression rules. Multiline record and list input may become one
canonical line. Reformatting the result is idempotent.

The formatter writes one space on each side of `%` and preserves its
precedence and left associativity with `*` and `/`.

Generic ADT declarations and function types use their ordinary source
presentation. Inferred covariant, contravariant, invariant, and irrelevant
parameter classifications do not add declaration modifiers or replace the
written type name. Function input and result types retain the same `fn(...) ->
...` presentation regardless of the variance used during type checking.

For a function declaration binder list, the formatter preserves the declared
ordinary type-binder order and an optional final effect-row binder. It writes
one space after each comma, removes a trailing comma, and does not add spaces
inside the angle brackets. An effect-only list uses the same rules. The
[source-surface specification](source-surface.md#function-declaration-binders)
defines which binder lists the parser accepts.

A handler header places `for` between its formatted parameter list and nominal
effect path, followed by any retained-effect list. Reformatting that output is
idempotent and preserves the same effect path and retained effects.

A directly written terminated handler list that is not part of an eligible
nested-chain migration keeps its body form and entry order. An expression body
begins on the `handle` header and retains any of its own multiline delimiters
before `with`. A block body begins on the next line.
Body lines and handler entries use one additional tab; `with` for a block and
the closing `end` align with `handle`. The formatter writes one entry per line,
writes commas only between entries, removes an input final comma, and preserves
comments attached to entries. A standalone comment before a block form's
`with` remains with the body, and a standalone comment before the terminating
`end` remains after the final entry. Formatting the result again produces
identical bytes.

The formatter migrates a comment-free chain of two or more directly nested
short handler expressions. It also migrates a directly nested chain that mixes
the short form with terminated expression-body handler lists. The complete
chain may span source lines and may use grouping parentheses, but its handled
body must format on one line. The result is one terminated expression-body
list whose entries run from the outermost handler to the innermost handler;
entries already grouped in one list retain their written order. The migration
preserves the handled body and each handler context expression. Reformatting
the migrated result produces identical bytes.

During one formatting pass, the formatter leaves the chain nested if it
contains comments, uses a handled block or `begin`, has a body that formats
across multiple lines, or reaches another handler only through a statement,
`defer`, call, branch, or postfix operator. An ineligible comment-free
multiline chain keeps a canonical multiline nested form that is byte-identical
on a second pass. Eligibility applies to the complete directly nested chain:
when an outer form is ineligible, no directly nested suffix of that chain
migrates on its own. A statement, `defer`, call, branch, or postfix operator
ends that direct chain, so an eligible chain inside the independent expression
may migrate without changing the ineligible outer form.

In structurally recognized variant-refinement type text, the formatter writes
one space on each side of `|`. It preserves the written alternative order,
duplicate alternatives, base qualification, and type-argument content. It
formats type arguments recursively: angle brackets have no adjacent inner
spaces, commas have no preceding space and one following space, and nested
generic arguments use the same rules at a refinement boundary. A malformed pipe
that is not part of a complete structural union is not normalized as a union;
in particular, `|>` remains one pipeline token during recovery. Nested generic
and explicit-call type arguments retain the closing angle brackets required by
each level. If a refinement has a surplus closing angle bracket, formatting
preserves it so a subsequent parse reports the same malformed boundary.

`begin` expressions and `defer` statements use one additional tab for each
nested body and align their closing `end` with the form's header. They retain
header and body comments. A comment following the closing `end` of an embedded
`begin` moves after the complete call, collection, record, postfix, decode, or
binary expression that continues from that `begin`.

A `match` line uses its parent indentation; arms use one deeper level and its
closing `end` aligns with the `match`. A two-arm boolean match becomes
`if`/`else`; a false continuation becomes `else if`. A boolean match
against string, integer, float, or unit literals becomes a literal `match`
with a wildcard fallback. Comments make a rewritable match lossless, so it is
left unchanged.

Standalone comments attach to the next parsed source line and receive that
line's indentation. Comment-only lines do not interrupt declarations. Trailing
comments stay on their source line. Formatting a preceding multiline handler
expression does not consume a trailing comment from the header of the next
adjacent body expression. Slash-prefixed comment-like text is not migrated.

## Limits and errors

The parse gate applies to the complete selected set; semantic analysis is not
a formatting gate. Formatting a module `let` declaration does not validate its
semantics. The shared
[command-analysis specification](command-analysis.md)
owns that boundary. After the parse gate, files are written sequentially. A
filesystem write error fails the command but does not roll back earlier
successful writes.

## References

Implementation: `crates/veln-cli/src/commands/fmt.rs`. The checked
[`explicit-import-alias`](../../examples/specification/fmt/explicit-import-alias/)
case covers alias placement and idempotence. Formatter golden tests cover
precedence, comments, schema spellings, multi-file atomicity, and other
idempotence boundaries. The checked
[`variance-presentation`](../../examples/specification/fmt/variance-presentation/)
case covers stable generic ADT and function type presentation for all four
inferred parameter classifications. The checked
[`generic-function-declaration-binders`](../../examples/specification/fmt/generic-function-declaration-binders/)
case covers ordinary and effect declaration binders, trailing-comma removal,
and idempotence. The checked
[`multiple-handler-expressions`](../../examples/specification/fmt/multiple-handler-expressions/)
case covers both terminated body forms, short-chain migration with and without
grouping parentheses, single-line and multiline chain layouts, mixed short and
terminated chains in both nesting directions, outermost-first entry order,
final-comma removal, boundary-comment retention, and idempotence. Formatter
golden tests also pin complete-chain eligibility: postfix, branch, `begin`,
handled-block, multiline-body, and comment boundaries keep the surrounding
mixed chain nested, while eligible independent chains beyond statement and
`defer` boundaries still migrate. Each boundary output is idempotent.
Golden coverage also places a commented mixed chain immediately after an
independently eligible mixed chain and checks that the second header keeps its
comment.
