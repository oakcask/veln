---
role: proposal
update-when: The proposed expression/block consistency model, conditional bodies, paired defer syntax, evaluation timing, canonical formatting, acceptance requirements, or readiness classification changes.
---

# Consistent Expression and Block Forms

Use one design model for single-expression forms, block forms, and constructs
that support both. The common requirements live in the
[Source Syntax Design Policy](../reference/source-syntax-design.md). This
proposal applies them to conditionals, arrow clauses, explicit scopes, and
paired `defer` forms.

The language changes are direct conditional branch bodies, unit conditionals
without a final `else`, and a new `defer Expr` statement equivalent to a
one-expression deferred block. Arrow clauses remain single-expression forms;
`begin ... end` remains the explicitly delimited expression for a sequence.

This page proposes unimplemented behavior. The current
[source surface](../specification/source-surface.md),
[types](../specification/types.md), and
[effects](../specification/effects.md) remain authoritative until implementation.
The acceptance tables below are planned evidence, not passing tests supplied
by this proposal. This is a Ready implementation target: the grammar,
classification, scope and evaluation tables, cleanup traces, and canonical
pairs below settle the language choices. Passing parser, backend, and
formatter checks is an implementation completion requirement.

## Design model and scope

Classification describes what a construct accepts, not how many physical lines
it occupies. One expression may contain a delimited multiline block. A body
contains statement lines; its syntax identifies its extent without leading
indentation. The design does not require every block construct to have a short
expression form.

| Construct | Classification and extent | Scope and evaluation |
| --- | --- | --- |
| `if` / `else if` / `else` | Block branches delimited by branch keywords and the conditional's `end`. | Each selected branch is a lexical cleanup scope; its result follows body-tail rules. |
| Match arm and handler operation clause | One `Expr` after `=>`; the expression starts on the arrow line. | Retain the expression's existing scopes and evaluation. A bare newline cannot start a sequence. |
| `begin ... end` | An expression containing an explicitly delimited body. | A distinct lexical cleanup scope with a body result. |
| `defer Expr` | One expression starting on the `defer` line. | Register the equivalent one-expression deferred block in the enclosing cleanup scope. |
| `defer NL Body end` | An explicitly delimited deferred body. | Same registration and cleanup rules as the short form; its body can contain local statements. |
| Function and test bodies | Existing block forms ending in `end`. | Retain existing parameter, result, effect, and cleanup rules; no new short form. |
| Handled body | Current `handle Expr with Handler(...) end` takes one expression, and `handle NL Body with Handlers end` provides an explicitly delimited block form. | Retain the handler and cleanup model defined by the [current source-surface specification](../specification/source-surface.md#executable-grammar). |

All new forms below are proposed, including `defer Expr`. Existing block-only
`defer` remains current behavior until implementation. This classification
prevents a short deferred expression from being mistaken for eager argument
capture, or a multiline arrow expression for an undelimited statement sequence.

## Motivation and usage

The current grammar uses `Expr` for every `if` branch. A sequence therefore
requires `begin ... end` inside a construct that already has `else` and `end`
boundaries. Direct branch bodies remove that redundant wrapper.

Arrow clauses have a different boundary. A bare `=>` followed by a sequence
would make the next sibling header or enclosing `end` terminate that sequence.
Readers would have to reconstruct the nesting to locate its end. Prefer forms
whose boundaries remain recognizable after indentation is removed. Keep an
arrow's right-hand side as one expression; an explicit `begin` gives a
statement sequence its own opening and closing tokens.

The proposed conditional and retained arrow forms are:

```veln
if enabled
	let x = compute()
	finish(x)
else
	default_value()
end

match value
	Some(x) => begin
		let y = transform(x)
		finish(y)
	end
	None => default_value()
end

handler fixed(seed: Int) for Ask
	value() => begin
		let result = seed + 1
		result
	end
end
```

These are source excerpts; surrounding declarations are omitted. `if`,
`else if`, and `else` have direct bodies. Arrow clauses retain their current
expression grammar, including expressions that have their own multiline
syntax, such as `begin`, `if`, and `match`.

This proposal also permits omission of the final `else` for unit-valued
conditionals. It does not add `if let`, guards, lambdas, loop syntax, general
operator continuation, or changes to contract and schema predicate grammars.
Function, test, and existing deferred-block syntax stays as specified today.
The new short `defer` form is defined below with the same cleanup semantics.

The current [handled block form](../specification/source-surface.md#executable-grammar)
provides a direct body for `handle`. The separate
[multiple-handler proposal](multiple-handler-syntax.md) retains only unfinished
migration and ordered-handler acceptance work. This proposal does not depend
on that remaining work. The direct bodies use the same lexical cleanup model.

## Proposed source contract

The following grammar replaces the conditional and deferred-statement
productions. `Expr` and body-line forms retain their existing meanings. `NL` here is a logical line
boundary, not a newline inside a grouped expression. Comments do not suppress
a logical newline; blank and comment-only lines are allowed between body lines.

```text
If         ::= "if" Expr NL BranchBody ElseIf* Else? "end"
ElseIf     ::= "else" "if" Expr NL BranchBody
Else       ::= "else" NL BranchBody
BranchBody ::= BodyLine+
BodyLine   ::= LetLine | DeferStatement | ExprLine
DeferStatement ::= "defer" Expr NL | "defer" NL Body "end" NL?
```

| Context | Body boundary at the current nesting level |
| --- | --- |
| `if` or `else if` branch | The next `else if`, `else`, or the conditional's `end`. |
| Final `else` branch | The conditional's `end`. |
| Short `defer` | One expression, followed by its logical newline; any block expression consumes its own delimiters. |
| Block `defer` | Its own `end`, after its nested expressions and blocks. |
| Arrow expression containing `begin` | The `begin` expression's own `end`, followed by the existing end of the clause's expression line. |

Nested expressions consume their own delimiters before an enclosing branch
boundary. Indentation is presentation, not a parsing boundary. Reindenting
valid input must not change body membership. This is a readability objective,
not a claim that arbitrary loss of delimiters can be recovered without nesting.

Match arms and handler operation clauses retain `=> Expr NL`. This proposal
does not add `=> NL Body` or a bare newline before the expression. A second
direct statement is rejected; put the sequence inside `=> begin ... end`.
Expressions such as `=> if ... end` keep their own delimiters and may use the
new conditional branch bodies. No sibling-header sequence parser is added.

Reject an empty written conditional branch, missing expression, or missing
enclosing delimiter. Use `()` for an intentional no-op. Comments and blank
lines alone do not satisfy the nonempty rule. An omitted `else` has the
separate meaning below. Existing arrow-clause syntax errors remain errors.

The same boundaries remain visible with leading indentation removed. For
example, the inner `end` closes `begin`, the next closes `match`, and the final
one closes `if` in this proposed input:

```veln
if enabled
match value
Some(x) => begin
let y = transform(x)
finish(y)
end
None => default_value()
end
else
default_value()
end
```

The parser must assign the same statements to each construct as in the
indented version. The formatter restores indentation without adding or
removing a scope.

Recovery must preserve a following valid sibling header and a following
top-level declaration whenever the missing boundary can be identified. An
unexpected enclosing boundary must not be consumed as a body expression.
Primary human diagnostics identify the failed fact at its source span;
related notes identify the enclosing construct and any repair guidance.

## Values, bindings, and cleanup

Every selected conditional branch is an implicit lexical cleanup scope. Its
binding, result, and cleanup semantics are those of `begin Body end` at that
location. This proposal adds no implicit cleanup scope to arrow clauses;
their explicit `begin` expressions retain the existing cleanup behavior.
An unselected branch performs no body work and registers no cleanup.

| Concern | Proposed observation |
| --- | --- |
| Evaluation | Execute direct body lines in source order. Evaluate the final value before scope cleanup. |
| Result | The final body line supplies the value if it is an expression. A final `let` or `defer` supplies `()`, as in an existing `begin` body. Earlier expressions do not supply the result. |
| Bindings | Earlier locals are visible to later lines in the same body. Body locals do not escape to sibling branches, conditions, or code after the conditional. Existing shadowing rules apply. |
| Type checking | Apply the current branch-result checking and expected-type propagation to the body result. |
| Registration | Direct `defer` is allowed in the new bodies. Capture registration-time bindings under the existing rules. A `defer` nested within a deferred block remains rejected. |
| Normal exit | Run the selected body's registered cleanup in reverse order before evaluation continues outside that body. |
| Exceptional exit | On `?` propagation, contract failure, runtime failure, or task cancellation, unwind each entered body scope using the existing cleanup guarantees, restrictions, and failure precedence. Later body expressions do not run. |
| Effects and handlers | Include body and deferred effects in existing inference. Cleanup observes handlers active at its scope exit. An inner handled expression restores its handlers before subsequent outer-body work; an outer handler stays active during body cleanup. |
| Explicit `begin` | Keep a distinct inner scope. Its cleanup completes before the next line in the enclosing body executes. |

For example, with a recording `record` operation, this body emits `inner`,
then `after`, then `outer` on normal completion:

```veln
if enabled
	defer
		record("outer")
	end
	begin
		defer
			record("inner")
		end
		()
	end
	record("after")
end
```

That example explains why a formatter must not remove an arbitrary `begin`.
Explicit nested scope is useful even when a surrounding construct has a body.
The same cleanup order remains available inside an explicit `begin` in a
match arm or operation clause.
Handler dispatch, child-task inheritance, scoped-handler restoration, and
resource-provider ownership do not acquire new semantics from this syntax.

## Paired defer forms

Retain `defer NL Body end` and add `defer Expr NL` in the same direct-body
positions. Both are statements, not expressions. A logical newline directly
after `defer` selects the block form; otherwise an expression must start on
that line. A trailing comment does not turn a newline into expression
continuation. Blank or comment-only lines after a block header remain inside
the block. Retain the existing allowance for an empty deferred block; an empty
short-form expression is invalid.

| Source shape | Proposed interpretation |
| --- | --- |
| `defer record(label) # comment` followed by `NL` | Short form; the comment belongs to this statement. |
| `defer # comment` followed by `NL record(label) NL end` | Block form; the header comment does not suppress the newline. |
| `defer NL # comment NL end` | Retained empty block with a body comment. |
| `defer let x = compute()` | Reject: `let` is not an expression. |
| `defer 1` | Reject: the deferred result is not unit. |
| `defer record(label)` followed by `NL record("body")` | One registration followed by an immediate independent statement. |

For every admissible expression `E`, the following have the same registration,
binding, effect, unit-result, and unwind behavior:

```text
defer E
```

```text
defer
    E
end
```

| Concern | Proposed rule for both forms |
| --- | --- |
| Registration | Register one cleanup when execution reaches the statement. Capture referenced local bindings using the existing registration-time snapshot and binding-identity rules. |
| Evaluation time | Evaluate all of `E` at cleanup, including the callee and its argument expressions. Registration does not execute those expressions. Capturing a local value does not recursively snapshot mutable resources reachable through it. |
| Result and effects | Require result type `()` and infer all deferred effects as for the current block. Do not silently discard a non-unit result. |
| Restrictions | Retain rejection of postfix `?` and nested `defer` anywhere in the deferred computation, including nested expression blocks. A bare `let` is not an expression operand. |
| Scope | Register in the nearest enclosing cleanup scope. The deferred computation has the same local scope in either spelling; the short form adds no additional enclosing cleanup boundary. |
| Failure and cancellation | Use existing reverse registration order, failure precedence, handler unwinding, and cancellation rules. A failure while evaluating a deferred argument occurs during cleanup, not registration. |
| Multiline expression | `defer begin ... end`, `defer if ... end`, and `defer match ... end` are single-expression forms with their expression's own delimiters. There is no additional short-form `end`. |

For example, these body lines register cleanup without calling `next_value`:

```veln
defer record_value(next_value())
record("body")
```

With recording test support, the expected trace is `body`, `next_value`, then
`record_value`. Replacing the first line with its one-expression deferred
block must preserve that trace. This is deliberately different from an API
that evaluates call arguments when registering the deferred call.

A second example distinguishes captured bindings from expression evaluation:

```veln
let label = "first"
defer record(label)
let label = "second"
record(label)
```

The expected trace is `second`, then `first`. A local declared only after the
registration point is unavailable in the deferred expression. The same
observations apply to block spelling. These examples are proposed excerpts
with separate Veln recording-handler support, not production effect selectors.

## Unit conditionals without a final else

An omitted final `else` means an implicit unit-valued fallback. Each written
branch must itself have result type `()`; the conditional also has type `()`.
It is not an implicit discard conversion for a non-unit value.

| Input | Proposed outcome |
| --- | --- |
| `if flag NL unit_call() NL end` | Accept; call only when `flag` is true; result `()`. |
| `if flag NL 1 NL end` | Reject the non-unit branch result, including when the caller discards the conditional's value. |
| Unit `if` / `else if` chain with no final `else` | Accept; evaluate conditions in order until one is true; do no body work when all are false. |
| Branch ends with `let x = compute()` and no final `else` | Accept with result `()` if the other checks succeed; `x` remains local. |
| Explicit `else NL ()` | Accept and retain that explicit branch in canonical output. |
| Explicit but empty `else` | Reject; omission and an empty written branch are distinct forms. |

Explicit final `else` keeps the current result compatibility rules. The
proposal does not weaken the requirement that each condition has type `Bool`.

## Canonical source and formatting

Use the existing tab indentation and comment spelling rules. `if`, `else if`,
`else`, and the conditional's `end` align; direct branch body lines use one
additional tab. Arrow expressions retain the current layout, including
`header => begin` with the explicit scope opener on the arrow line.

| Accepted input shape | Canonical output |
| --- | --- |
| One or several lines in an `if` branch | Direct body lines; never insert a `begin` wrapper. |
| Nested conditional in a branch | Indent the complete nested conditional one level; retain its own `else` and `end`. |
| Explicit `begin` as a branch expression | Preserve it and its closing `end`; do not splice its body into the branch scope. |
| `header => Expr` | Retain the current single-expression layout. |
| `header => begin` with several statements | Keep `begin` on the arrow line and its own `end` at the header's indentation. |
| `header => if` or `header => match` | Keep the nested expression's opener on the arrow line and retain its closing delimiter. |
| Short `defer Expr` | Keep the expression opener on the `defer` line, including a multiline block expression with its own `end`. |
| Block `defer` | Keep the header, body indentation, and explicit closing `end`, even for a one-expression or empty body. |
| Explicit `else NL ()` | Keep the branch; do not infer that the author intended omission. |

For example, this accepted input:

```veln
if enabled
  let x = compute()
  if ready(x)
    finish(x)
  else
    default_value()
  end
else
  default_value()
end
```

formats as:

```veln
if enabled
	let x = compute()
	if ready(x)
		finish(x)
	else
		default_value()
	end
else
	default_value()
end
```

The match and handler excerpts in Motivation are already canonical. In
particular, formatting must not turn `=> begin` into `=> NL` followed by a
bare sequence, or move `begin` to a separate line after a bare arrow.
An existing `if` branch containing `begin` retains it as a nested expression:

```veln
if enabled
	begin
		let x = compute()
		finish(x)
	end
else
	default_value()
end
```

Both direct-body and explicit-scope branch spellings remain accepted. They
do not converge by removing explicit scopes. Ordinary formatting preserves
those boundaries; manual removal of a scope is a separate semantic edit.

Both `defer` spellings are canonical by design. Ordinary formatting does not
expand a short expression or collapse a block, even where they are equivalent.
Preserving the chosen form keeps the visible statement/block distinction and
comment boundaries stable as code is edited. For example:

```veln
defer record(label)
defer
	record(label)
end
defer begin
	let message = make_message(label)
	record(message)
end
```

These three statements are already canonical. The last `end` closes the
`begin` expression, not an additional deferred block. A short `defer` followed
by another body line never absorbs that line. Space-indented versions of each
form converge to the same tab-indented structure shown above, without changing
which form was chosen. There is no automatic scope-removal or defer-form
migration and no new line-width heuristic.

Preserve evaluation count and order, binding identity, value, inferred effects,
cleanup timing, handler restoration, and source comments. Header comments stay
on their headers; leading body comments remain with the body; trailing
expression comments remain with that expression. Preserve comments attached to
`begin`, `else`, and `end`. Blank lines and comments between branches stay
between the same branches. If a layout conversion cannot preserve attachment,
retain the affected layout rather than move or discard the comment.

Inputs with the same body structure and comment attachments converge to the
same indentation and token spacing. Explicit scopes, the chosen deferred form,
and explicit unit fallback branches are intentional structural differences, not convergence targets.
Formatting twice produces identical bytes. Retain the current
[format command](../specification/command-fmt.md) parsing gate and write policy:
a parse failure cannot cause a partial rewrite of the selected files.
Recovered malformed bodies are not migration input.

## Related work and alternatives

The primary sources below were inspected for this proposal. External reference
lookups used fresh official documentation after the local bibliography cache
had no matching body-syntax records. They support design comparisons, not
claims that Veln implements the proposed forms.

- [The Rust Reference: Match expressions](https://doc.rust-lang.org/reference/expressions/match-expr.html)
  admits block expressions after an arm arrow and limits pattern bindings to
  the arm. This supports retaining a result-producing block expression after
  an arrow. Rust uses braces and has different ownership rules; Veln retains
  explicit `begin ... end` for sequences. This is comparative support for visible block
  boundaries, not authority for Veln cleanup semantics.
- [Scala 3 Reference: Optional Braces](https://docs.scala-lang.org/scala3/reference/other-new-features/indentation.html#special-treatment-of-case-clauses)
  defines indentation regions and special case-clause boundaries. It shows
  another way to present clause bodies without braces. This proposal rejects
  indentation-based arrow bodies: Veln must remain readable without indentation
  cues. Conditional branches keep explicit `else` and `end` boundaries, and arrow
  sequences keep `begin ... end`.
- Veln [PR #1688](https://github.com/oakcask/veln/pull/1688),
  `feat(language): add deferred cleanup source surface`, introduced explicit
  lexical cleanup forms and registration-point binding checks. Its inspected
  description motivates preserving binding identity and scope-aware tooling.
  The PR described a static-only intermediate stage; current cleanup behavior
  comes from the current specifications, not that historical limitation.
  This proposal retains explicit scopes and extends direct-body placement.
- [The Go Programming Language Specification: Defer statements](https://go.dev/ref/spec#Defer_statements)
  restricts a deferred expression to a call and evaluates its function and
  arguments at registration. This illustrates why a short spelling alone does
  not specify timing. Veln instead adopts any unit-valued admissible expression
  and preserves its existing block semantics: only binding snapshots occur at
  registration, and the expression evaluates during lexical cleanup. Go's
  function-return boundary is not adopted as Veln's cleanup boundary.

| Alternative | Decision |
| --- | --- |
| Require `begin` for conditional branch sequences | Reject: `else` and `end` already delimit those bodies. |
| Give every construct both forms | Reject: classification does not require a new short form for every block. Add short `defer` for one deferred expression; keep arrows expression-only and conditionals block-based. |
| Evaluate short deferred-call arguments at registration | Reject: it would give short and block forms different evaluation and failure timing. |
| Permit `=> NL` followed by a bare sequence | Reject: without indentation cues, readers must find the next sibling header to infer the sequence boundary. |
| Make every arrow body a sequence even on the arrow line | Reject: retain one expression whose own syntax identifies any multiline body. |
| Add significant indentation or a per-arm `end` | Reject: explicit `begin ... end` already provides the required sequence boundary. |
| Flatten explicit `begin` during formatting | Reject: explicit scope should remain visible, and arbitrary flattening can change cleanup timing. |
| Permit non-unit branches when `else` is omitted | Reject: silent value discard would obscure whether a conditional produces a meaningful result. |

## Acceptance model and delivery

All evidence in this table is required future implementation work. The grammar,
boundary table, value/scope table, explicit traces, and formatting pairs above
are review oracles independent of the implementation under test.

| Case | Required observation | Planned verification |
| --- | --- | --- |
| Branch bodies | Accept `let`, expressions, and direct `defer` in every branch of an `if` / `else if` / `else` chain. Return the final expression or unit for a non-expression final line. | Executable source grammar, accepted fixtures, syntax and semantic tests. |
| Paired deferred syntax | Accept short expressions and retained blocks in direct-body positions, including comments after headers, empty retained blocks, nested block expressions, and subsequent independent statements. Reject missing expressions, non-unit results, `?`, nested registration, and bare `let` operands. | Grammar fixtures, parser recovery and semantic diagnostic tests. |
| Deferred timing | Assert `body`, `next_value`, `record_value` and the shadowing trace `second`, `first` for both spellings. A failing deferred argument must fail during cleanup; remaining registered cleanup is still attempted. | Separate scoped Veln recording handlers and backend trace assertions on normal, failure, and cancellation exits. |
| Arrow boundary preservation | Retain `=> Expr`, `=> begin`, and nested `=> if` / `=> match`. Reject bare newline bodies and additional direct statements outside an explicit expression. | Existing arrow grammar fixtures plus rejected syntax and formatter regression cases. |
| Indentation independence | Removing leading whitespace from nested conditional, both deferred forms, and `=> begin` examples preserves statement ownership, explicit scope boundaries, and cleanup traces. Reviewers can identify sequence openers and closers from tokens alone. | Parser comparison ignoring source spans, backend trace assertions, and source-form review. |
| Boundaries | A nested match, conditional, explicit scope, deferred block, grouped expression, and handled expression each consume their own boundaries. A `satisfy` arrow inside a branch remains part of its expression. Reindentation preserves membership. | Parser trees and formatter fixtures, including direct handled-block combinations. |
| Empty and malformed input | Reject empty written conditional branches, missing deferred expressions, missing arrows, and missing enclosing delimiters. Preserve recoverable sibling headers, the next declaration, all lossless tokens, and human diagnostic context. | Rejected grammar fixtures, parser recovery tests, CLI human diagnostic examples. |
| Scope and result context | Reject use of one body's locals from another body or after the construct. Preserve expected result types and constructor refinement context, including a conditional inside a match arm or operation clause. | Semantic tests, navigation and rename tests, AST wire round trips. |
| Optional else | Accept unit results and unit chains; reject a non-unit branch even in a discarded expression position. Preserve an explicit unit fallback. | Semantic tests and compiler diagnostic examples. |
| Selection and sequencing | Only the selected body runs, conditions and body lines run in order, and the result is evaluated once before cleanup. | Backend integration examples with explicit return values and effect traces. |
| Cleanup | Emit `inner`, `after`, `outer` for the example above; reverse multiple registrations; do not register unselected cleanup. Repeat for a conditional inside an explicit arm or operation-clause scope. | Backend deferred-cleanup tests with separate scoped Veln recording handlers. |
| Early exit and failure | On `?`, contract/runtime failure, and cancellation, skip later lines and unwind entered scopes. Attempt later cleanup after cleanup failure and preserve the current primary-failure rules. | Backend propagation, failure-precedence, cancellation, and effect-injection tests. |
| Handler boundaries | Cleanup inside a handled body sees its handler; enclosing cleanup sees restored handlers after inner exit. Child tasks and provider-bound resources retain existing inheritance and identity. | Existing effect-boundary assertions plus focused new body combinations; production handlers use real host defaults. |
| Canonical output | Check single/multiple-line branches and nested control flow; retain existing arrow expression layouts and both canonical defer forms. Verify the exact before/after pairs above. | Formatter golden tests and checked CLI format cases. |
| Preservation and stability | Check actual formatter output for unchanged values, binding identity, effects, cleanup and handler traces. Preserve delimiter/header/body comments, retain explicit scopes, converge inputs with the same scope structure, and obtain identical bytes on the second format. | Formatter comment and idempotence fixtures plus semantic/backend checks on formatter-produced source. |
| Parse-before-write | A malformed selected file leaves every selected file unchanged. | Checked multi-file CLI formatting case. |

Implementation must cover parsing, recovery, lossless syntax, AST and wire
representation, semantic checking, checked-core and typed-IR lowering, backend
cleanup regions, formatting, metrics, documentation extraction, editor scopes,
and published MCP language-reference rendering. Implicit body scopes must
remain visible to navigation and rename conflict checking. A body must not
inherit implementation-dependent scope behavior from an accidental wrapper.

Use syntax and semantic suites for language contracts. New
`examples/specification/` cases target independently identified compiler,
formatter, diagnostic, or backend contracts; they do not substitute for stdlib
API companion tests. Follow the [test placement policy](../reference/toolchain-test-harness.md)
and scoped Veln fake-handler policy. Run broad and generated-input checks
through the guarded local runners. Include bounded nested-body analysis checks
and apply the performance-regression audit if implementation changes analysis
of large generated inputs.

## Readiness and completion

The declared forms have explicit token boundaries, result and cleanup scopes,
deferred evaluation timing, and canonical output. Ready selection uses these
reviewable contracts and the acceptance model above; it does not require a
parser or formatter prototype. The independent
[Source Syntax Design Policy](../reference/source-syntax-design.md) continues
to govern later syntax changes.

On delivery, update executable grammar, checked examples, and the
source-surface, type, effect, execution, format, and affected tooling
specifications together. Remove this proposal and its catalog entry after the
complete language contract is implemented. Current language specifications
and executable examples are unchanged by this proposal because the conditional
and short-defer behavior is not implemented.
