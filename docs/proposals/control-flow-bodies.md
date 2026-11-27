---
role: proposal
update-when: The proposed if branch and arrow body grammar, implicit cleanup scopes, optional else typing, canonical formatting, or implementation acceptance requirements change.
---

# Bodies for Conditional Branches and Arrow Clauses

Allow `if` branches, match arms, and handler operation clauses to contain
direct body lines without a mandatory `begin` wrapper. Keep `begin ... end`
as an explicit nested lexical and cleanup scope.

This page proposes unimplemented behavior. The current
[source surface](../specification/source-surface.md),
[types](../specification/types.md), and
[effects](../specification/effects.md) remain authoritative until implementation.
The acceptance tables below are planned evidence, not passing tests supplied
by this PR. The proposal is in Design In Progress pending review of the
boundary, scope, and formatting decisions specified here.

## Motivation and scope

The current grammar uses `Expr` for every `if` branch, match arm, and handler
operation result. Multiple operations require a `begin` expression even where
the surrounding syntax can delimit a body. This conflates the need for a
statement sequence with the need for an explicit nested scope.

The proposed forms are:

```veln
if enabled
	let x = compute()
	finish(x)
else
	default_value()
end

match value
	Some(x) =>
		let y = transform(x)
		finish(y)
	None => default_value()
end

handler fixed(seed: Int) for Ask
	value() =>
		let result = seed + 1
		result
end
```

These are source excerpts; surrounding declarations are omitted. Handler
operation clauses and match arms use the same choice between an expression
on the arrow line and a body after a newline. `if`, `else if`, and `else`
already introduce body lines on the next line.

This proposal also permits omission of the final `else` for unit-valued
conditionals. It does not add `if let`, guards, lambdas, loop syntax, general
operator continuation, or changes to contract and schema predicate grammars.
Function, test, and deferred-block syntax stays as specified today.

The separate [multiple-handler proposal](multiple-handler-syntax.md) defines
a direct body for `handle`. On the base revision of this proposal, that form
is still proposed rather than implemented. Neither proposal requires the
other to land first. Their bodies use the same lexical cleanup model.

## Proposed source contract

The following grammar replaces only the listed productions. `Expr`,
`Pattern`, `Name`, `HandlerOperationParams`, and body-line forms retain their
existing meanings. `NL` here is a logical line boundary, not a newline inside
a grouped expression. Comments do not suppress a logical newline; blank and
comment-only lines are allowed between body lines.

```text
If                     ::= "if" Expr NL BranchBody ElseIf* Else? "end"
ElseIf                 ::= "else" "if" Expr NL BranchBody
Else                   ::= "else" NL BranchBody
BranchBody             ::= BodyLine+
BodyLine               ::= LetLine | DeferStatement | ExprLine
MatchArm               ::= Pattern "=>" ArrowBody
HandlerOperationClause ::= Name "(" HandlerOperationParams? ")" "=>" ArrowBody
ArrowBody              ::= Expr NL | NL BranchBody
```

`BranchBody` is a sequence with a context-specific stopping boundary, defined
below. It introduces no additional closing token. An inline arrow expression
may itself be multiline, such as `=> if ... end` or `=> begin ... end`.
Such an expression consumes its own delimiters before its final `NL`.

| Context | Body boundary at the current nesting level |
| --- | --- |
| `if` or `else if` branch | The next `else if`, `else`, or the conditional's `end`. |
| Final `else` branch | The conditional's `end`. |
| Match arm after `=> NL` | The next logical line starting with a complete `Pattern` immediately followed by `=>`, or the match's `end`. |
| Handler operation after `=> NL` | The next logical line starting with a complete operation header `Name(params) =>`, or the handler declaration's `end`. |

Only an entire header prefix at the start of a logical line terminates an
arrow body. An arrow in `let candidate = _ satisfy x => predicate` does not
start another arm. Tokens within calls, aggregates, nested `if`, `match`,
`begin`, `defer`, or handled expressions belong to those nested constructs.
Their headers and closing tokens do not terminate the surrounding body.
Indentation is presentation, not a parsing boundary. Reindenting valid input
must not change its body membership.

Inline arrow syntax admits exactly one expression. A second direct body line
requires a newline immediately after `=>`; it cannot silently extend an
inline arm. Reject an empty branch, empty arrow body, missing expression,
missing `=>`, or missing enclosing delimiter. Use `()` for an intentional
no-op. Comments and blank lines alone do not satisfy the nonempty rule.
This applies to empty branches explicitly written in source; an omitted
`else` has the separate meaning below.

Recovery must preserve a following valid sibling header and a following
top-level declaration whenever the missing boundary can be identified. An
unexpected enclosing boundary must not be consumed as a body expression.
Primary human diagnostics identify the failed fact at its source span;
related notes identify the enclosing construct and any repair guidance.

## Values, bindings, and cleanup

Every selected branch or arrow body is an implicit lexical cleanup scope,
including an inline arrow expression. A multiline body's binding, result,
and cleanup semantics are those of `begin Body end` at that location.
An unselected branch performs no body work and registers no cleanup.

| Concern | Proposed observation |
| --- | --- |
| Evaluation | Execute direct body lines in source order. Evaluate the final value before scope cleanup. |
| Result | The final body line supplies the value if it is an expression. A final `let` or `defer` supplies `()`, as in an existing `begin` body. Earlier expressions do not supply the result. |
| Bindings | Earlier locals are visible to later lines in the same body. Body locals do not escape to sibling branches, subsequent clauses, conditions, or code after the enclosing construct. Existing shadowing rules apply. |
| Header bindings | Match pattern bindings and handler operation parameters remain visible throughout their own body and retain their current identities and type context. |
| Type checking | Apply the current branch-result checking and expected-type propagation to the body result. A handler body's result must satisfy its operation result type. |
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
The same cleanup order is required for a match arm and an operation clause.
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
additional tab. Match headers use one level below `match`; multiline arm
bodies use one further level. Handler headers use one level below `handler`;
their multiline bodies use one further level. There is no arm-specific `end`.

| Accepted input shape | Canonical output |
| --- | --- |
| One or several lines in an `if` branch | Direct body lines; never insert a `begin` wrapper. |
| Inline arrow expression that formats on one line | Keep `header => Expr` on one line. There is no new line-width heuristic. |
| Arrow body containing exactly one expression that formats on one line | Converge to `header => Expr`, unless a comment attachment requires the body layout. |
| Arrow body with a `let`, `defer`, or multiple direct body lines | `header =>`, newline, indented body lines. |
| Inline arrow expression whose canonical output spans multiple lines | Move the whole expression below `header =>` as a one-expression body. Keep the expression's own delimiters. |
| Explicit `begin`, including a sole expression in a branch or arm | Preserve it and its closing `end`; do not splice its body into the enclosing scope. |
| Explicit `else NL ()` | Keep the branch; do not infer that the author intended omission. |

For example, this accepted one-expression body:

```veln
match value
	Some(x) =>
		transform(x)
	None => 0
end
```

formats as:

```veln
match value
	Some(x) => transform(x)
	None => 0
end
```

An existing multiline expression on the arrow line:

```veln
match value
	Some(x) => begin
		let y = transform(x)
		finish(y)
	end
	None => 0
end
```

formats as:

```veln
match value
	Some(x) =>
		begin
			let y = transform(x)
			finish(y)
		end
	None => 0
end
```

Apply the same rules to handler operation clauses. A multiline nested `if`
or `match` is similarly moved below the arrow without inserting `begin`.
Existing `if` branches containing a `begin` retain it as a nested expression.
Ordinary formatting performs these layout conversions; it does not remove
explicit scopes. Manual removal of a scope is a separate semantic edit.

Preserve evaluation count and order, binding identity, value, inferred effects,
cleanup timing, handler restoration, and source comments. A comment following
`=>` stays on that header; leading body comments remain with the body; trailing
expression comments remain with that expression. Do not collapse a body when
that would change any attachment. Preserve comments on `begin` and `end`.
Blank lines and comments between arms remain between the same arms.

Comment-sensitive bodies are the intentional exception to single-expression
convergence. Otherwise equivalent inline and one-expression body forms produce
identical canonical output. Formatting twice produces identical bytes.
Retain the current [format command](../specification/command-fmt.md) parsing
gate and write policy: a parse failure cannot cause a partial rewrite of the
selected files. Recovered malformed bodies are not migration input.

## Related work and alternatives

The primary sources below were inspected for this proposal. External reference
lookups used fresh official documentation after the local bibliography cache
had no matching body-syntax records. They support design comparisons, not
claims that Veln implements the proposed forms.

- [The Rust Reference: Match expressions](https://doc.rust-lang.org/reference/expressions/match-expr.html)
  admits block expressions after an arm arrow and limits pattern bindings to
  the arm. This supports keeping a local result-producing body per arm. Rust
  uses braces and has different ownership rules; Veln instead uses a newline
  and sibling-header boundaries. The inference here is that arm scope need
  not require a separate user-written block wrapper.
- [Scala 3 Reference: Optional Braces](https://docs.scala-lang.org/scala3/reference/other-new-features/indentation.html#special-treatment-of-case-clauses)
  defines indentation regions and special case-clause boundaries. It shows
  another way to present clause bodies without braces. This proposal adopts
  the uncluttered body layout but rejects significant indentation: Veln keeps
  explicit enclosing `end` tokens and syntactically recognizable headers.
- Veln [PR #1688](https://github.com/oakcask/veln/pull/1688),
  `feat(language): add deferred cleanup source surface`, introduced explicit
  lexical cleanup forms and registration-point binding checks. Its inspected
  description motivates preserving binding identity and scope-aware tooling.
  The PR described a static-only intermediate stage; current cleanup behavior
  comes from the current specifications, not that historical limitation.
  This proposal retains explicit scopes and extends direct-body placement.

| Alternative | Decision |
| --- | --- |
| Require `begin` for all multiple-line bodies | Reject: it makes ordinary body sequencing require a separate explicit scope spelling. |
| Change only `if` | Reject: it leaves match arms and operation clauses with the same sequencing inconvenience. |
| Make every arrow body a sequence even on the arrow line | Reject: a newline after `=>` makes the sequence boundary visible and preserves the inline single-expression form. |
| Add significant indentation or a per-arm `end` | Reject: sibling headers and the enclosing `end` provide boundaries without either additional mechanism. |
| Flatten explicit `begin` during formatting | Reject: explicit scope should remain visible, and arbitrary flattening can change cleanup timing. |
| Permit non-unit branches when `else` is omitted | Reject: silent value discard would obscure whether a conditional produces a meaningful result. |

## Acceptance model and delivery

All evidence in this table is required future implementation work. The grammar,
boundary table, value/scope table, explicit traces, and formatting pairs above
are review oracles independent of the implementation under test.

| Case | Required observation | Planned verification |
| --- | --- | --- |
| Branch bodies | Accept `let`, expressions, and direct `defer` in every branch of an `if` / `else if` / `else` chain. Return the final expression or unit for a non-expression final line. | Executable source grammar, accepted fixtures, syntax and semantic tests. |
| Arrow choice | Accept inline expressions and newline bodies for both match and handler operations. Reject extra direct lines after an inline expression. | Accepted/rejected grammar fixtures and parser tests. |
| Boundaries | A nested match, conditional, explicit scope, deferred block, grouped expression, and handled expression each consume their own boundaries. A `satisfy` arrow inside a body does not create an arm. Reindentation preserves membership. | Parser trees and formatter fixtures; add direct handled-block combinations when the multiple-handler feature exists. |
| Empty and malformed input | Reject empty bodies, missing arrows, and missing enclosing delimiters. Preserve recoverable sibling headers, the next declaration, all lossless tokens, and human diagnostic context. | Rejected grammar fixtures, parser recovery tests, CLI human diagnostic examples. |
| Scope and result context | Reject use of one body's locals from another body or after the construct. Preserve header binding identity, expected result types, and constructor refinement context. | Semantic tests, navigation and rename tests, AST wire round trips. |
| Optional else | Accept unit results and unit chains; reject a non-unit branch even in a discarded expression position. Preserve an explicit unit fallback. | Semantic tests and compiler diagnostic examples. |
| Selection and sequencing | Only the selected body runs, conditions and body lines run in order, and the result is evaluated once before cleanup. | Backend integration examples with explicit return values and effect traces. |
| Cleanup | Emit `inner`, `after`, `outer` for the example above; reverse multiple registrations; do not register unselected cleanup. Repeat in an arm and operation clause. | Backend deferred-cleanup tests with separate scoped Veln recording handlers. |
| Early exit and failure | On `?`, contract/runtime failure, and cancellation, skip later lines and unwind entered scopes. Attempt later cleanup after cleanup failure and preserve the current primary-failure rules. | Backend propagation, failure-precedence, cancellation, and effect-injection tests. |
| Handler boundaries | Cleanup inside a handled body sees its handler; enclosing cleanup sees restored handlers after inner exit. Child tasks and provider-bound resources retain existing inheritance and identity. | Existing effect-boundary assertions plus focused new body combinations; production handlers use real host defaults. |
| Canonical output | Check all formatting-table rows for both arrow contexts, single/multiple lines, and nested control flow. Verify the exact before/after pairs above. | Formatter golden tests and checked CLI format cases. |
| Preservation and stability | Check actual formatter output for unchanged values, binding identity, effects, cleanup and handler traces. Preserve delimiter/header/body comments, retain explicit scopes, converge eligible forms, and obtain identical bytes on the second format. | Formatter comment and idempotence fixtures plus semantic/backend checks on formatter-produced source. |
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
