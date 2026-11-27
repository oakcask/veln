---
role: proposal
update-when: The proposed if branch body grammar, explicit arrow-expression boundaries, implicit cleanup scopes, optional else typing, canonical formatting, or implementation acceptance requirements change.
---

# Bodies for Conditional Branches

Allow `if` branches to contain direct body lines without a mandatory `begin`
wrapper. Retain single-expression match arms and handler operation clauses.
Use `begin ... end` for a statement sequence in an arrow expression and for
an explicit nested lexical and cleanup scope.

This page proposes unimplemented behavior. The current
[source surface](../specification/source-surface.md),
[types](../specification/types.md), and
[effects](../specification/effects.md) remain authoritative until implementation.
The acceptance tables below are planned evidence, not passing tests supplied
by this PR. The proposal is in Design In Progress pending review of the
boundary, scope, and formatting decisions specified here.

## Motivation and scope

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
Function, test, and deferred-block syntax stays as specified today.

The separate [multiple-handler proposal](multiple-handler-syntax.md) defines
a direct body for `handle`. On the base revision of this proposal, that form
is still proposed rather than implemented. Neither proposal requires the
other to land first. Their bodies use the same lexical cleanup model.

## Proposed source contract

The following grammar replaces only the conditional productions. `Expr` and
body-line forms retain their existing meanings. `NL` here is a logical line
boundary, not a newline inside a grouped expression. Comments do not suppress
a logical newline; blank and comment-only lines are allowed between body lines.

```text
If         ::= "if" Expr NL BranchBody ElseIf* Else? "end"
ElseIf     ::= "else" "if" Expr NL BranchBody
Else       ::= "else" NL BranchBody
BranchBody ::= BodyLine+
BodyLine   ::= LetLine | DeferStatement | ExprLine
```

| Context | Body boundary at the current nesting level |
| --- | --- |
| `if` or `else if` branch | The next `else if`, `else`, or the conditional's `end`. |
| Final `else` branch | The conditional's `end`. |
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

Both direct-body and explicit-scope spellings remain accepted. They do not
converge by removing explicit scopes. Ordinary formatting preserves those
boundaries; manual removal of a scope is a separate semantic edit. There is
no automatic scope-removal migration and no new line-width heuristic.

Preserve evaluation count and order, binding identity, value, inferred effects,
cleanup timing, handler restoration, and source comments. Header comments stay
on their headers; leading body comments remain with the body; trailing
expression comments remain with that expression. Preserve comments attached to
`begin`, `else`, and `end`. Blank lines and comments between branches stay
between the same branches. If a layout conversion cannot preserve attachment,
retain the affected layout rather than move or discard the comment.

Inputs with the same body structure and comment attachments converge to the
same indentation and token spacing. Explicit scopes and explicit unit fallback
branches are intentional structural differences, not convergence targets.
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
  an arrow. Rust uses braces and has different ownership rules; Veln retains explicit
  `begin ... end` for sequences. This is comparative support for visible block
  boundaries, not authority for Veln cleanup semantics.
- [Scala 3 Reference: Optional Braces](https://docs.scala-lang.org/scala3/reference/other-new-features/indentation.html#special-treatment-of-case-clauses)
  defines indentation regions and special case-clause boundaries. It shows
  another way to present clause bodies without braces. This proposal rejects
  indentation-based arrow bodies: Veln must remain readable without indentation
  cues. Conditional branches keep explicit
  `else` and `end` boundaries, and arrow sequences keep `begin ... end`.
- Veln [PR #1688](https://github.com/oakcask/veln/pull/1688),
  `feat(language): add deferred cleanup source surface`, introduced explicit
  lexical cleanup forms and registration-point binding checks. Its inspected
  description motivates preserving binding identity and scope-aware tooling.
  The PR described a static-only intermediate stage; current cleanup behavior
  comes from the current specifications, not that historical limitation.
  This proposal retains explicit scopes and extends direct-body placement.

| Alternative | Decision |
| --- | --- |
| Require `begin` for conditional branch sequences | Reject: `else` and `end` already delimit those bodies. |
| Change only conditional branches | Adopt: conditionals already have explicit branch delimiters, while arrow sequences benefit from their own `begin ... end`. |
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
| Arrow boundary preservation | Retain `=> Expr`, `=> begin`, and nested `=> if` / `=> match`. Reject bare newline bodies and additional direct statements outside an explicit expression. | Existing arrow grammar fixtures plus rejected syntax and formatter regression cases. |
| Indentation independence | Removing leading whitespace from nested conditional and `=> begin` examples preserves statement ownership, explicit scope boundaries, and cleanup traces. Reviewers can identify sequence openers and closers from tokens alone. | Parser comparison ignoring source spans, backend trace assertions, and source-form review. |
| Boundaries | A nested match, conditional, explicit scope, deferred block, grouped expression, and handled expression each consume their own boundaries. A `satisfy` arrow inside a branch remains part of its expression. Reindentation preserves membership. | Parser trees and formatter fixtures; add direct handled-block combinations when the multiple-handler feature exists. |
| Empty and malformed input | Reject empty bodies, missing arrows, and missing enclosing delimiters. Preserve recoverable sibling headers, the next declaration, all lossless tokens, and human diagnostic context. | Rejected grammar fixtures, parser recovery tests, CLI human diagnostic examples. |
| Scope and result context | Reject use of one body's locals from another body or after the construct. Preserve expected result types and constructor refinement context, including a conditional inside a match arm or operation clause. | Semantic tests, navigation and rename tests, AST wire round trips. |
| Optional else | Accept unit results and unit chains; reject a non-unit branch even in a discarded expression position. Preserve an explicit unit fallback. | Semantic tests and compiler diagnostic examples. |
| Selection and sequencing | Only the selected body runs, conditions and body lines run in order, and the result is evaluated once before cleanup. | Backend integration examples with explicit return values and effect traces. |
| Cleanup | Emit `inner`, `after`, `outer` for the example above; reverse multiple registrations; do not register unselected cleanup. Repeat for a conditional inside an explicit arm or operation-clause scope. | Backend deferred-cleanup tests with separate scoped Veln recording handlers. |
| Early exit and failure | On `?`, contract/runtime failure, and cancellation, skip later lines and unwind entered scopes. Attempt later cleanup after cleanup failure and preserve the current primary-failure rules. | Backend propagation, failure-precedence, cancellation, and effect-injection tests. |
| Handler boundaries | Cleanup inside a handled body sees its handler; enclosing cleanup sees restored handlers after inner exit. Child tasks and provider-bound resources retain existing inheritance and identity. | Existing effect-boundary assertions plus focused new body combinations; production handlers use real host defaults. |
| Canonical output | Check single/multiple-line branches and nested control flow; retain existing arrow expression layouts. Verify the exact before/after pairs above. | Formatter golden tests and checked CLI format cases. |
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

Promote this target to Ready after review resolves the declared design choices
without leaving grammar or formatter behavior to implementation. On delivery,
update the executable grammar, checked examples, and the source-surface, type,
effect, execution, format, and affected tooling specifications together. Remove
this proposal and its catalog entry after the complete contract is implemented.
This proposal PR does not change current specifications or executable examples:
the new syntax and optional-else semantics are not implemented yet.
