---
role: proposal
update-when: The proposed multiple-handler expression syntax, canonical formatting and automatic migration, handler ordering, context evaluation, scope preservation, or implementation acceptance gates change.
---

# Multiple-handler Expressions

Allow one `handle` expression to apply an ordered list of handlers. This
removes repeated `handle` prefixes and indentation while preserving the
behavior of explicitly nested single-handler expressions.

This proposal defines unimplemented behavior. The current
[effects specification](../specification/effects.md) and
[source surface](../specification/source-surface.md) remain authoritative until
implementation. The design is ready for implementation; the acceptance cases
below are planned evidence, not tests supplied or passing with this proposal.

## Motivation and usage

The `execute_read` function in
[`io_direct_validation_test.veln`](../../crates/veln-stdlib/veln/io_direct_validation_test.veln)
currently applies eight handlers to one body. Its source has eight consecutive
`handle` prefixes followed by eight `with` clauses. Formatting that expression
as nested blocks still makes the reader count matching scopes.

The proposed equivalent expression is:

```veln
handle read_body(owner, 920, state, maximum) with
	io_provider_test::mutate(calls.tx),
	io_provider_test::sync(calls.tx),
	io_provider_test::close(calls.tx),
	scripted_write_at(owner, 920, state),
	scripted_read_at(owner, 920, state),
	io_provider_test::seek(calls.tx),
	scripted_write(owner, 920, state),
	scripted_read(owner, 920, state)
end
```

The entries above reverse the existing `with` clause order: the first entry
is outermost. This preserves the original nesting even where context
evaluation or a handler's own effects make order observable.

A block body avoids a separate `begin` expression:

```veln
handle
	let input = read_input()
	process(input)
with
	input_handler(),
	output_handler()
end
```

These examples illustrate proposed syntax; they are excerpts with surrounding
declarations omitted.

## Source contract

Retain `handle Expr with Handler(arguments)` as a single-handler expression
without a closing `end`. Add two explicitly terminated forms:

```text
Handle             ::= SingleHandle | ExpressionHandle | BlockHandle
SingleHandle       ::= "handle" Expr "with" HandlerApplication
ExpressionHandle   ::= "handle" Expr "with" NL HandlerEntries "end"
BlockHandle        ::= "handle" NL Body "with" NL HandlerEntries "end"
HandlerApplication ::= MemberPath "(" ArgList? ")"
HandlerEntries     ::= HandlerApplication ("," NL HandlerApplication)* ","? NL
```

`Expr`, `Body`, `MemberPath`, `ArgList`, and `NL` retain their source-surface
meanings. Blank lines and comments follow the existing block conventions.
Newlines inside an argument list follow the existing call-argument rules.

| Surface | Proposed rule |
| --- | --- |
| Newline immediately after `handle` | Starts a block body. The body has the binding, result, and cleanup behavior of `begin Body end`. |
| Newline immediately after `with` | Starts a nonempty handler list terminated by its own `end`. |
| List cardinality | One or more entries; a one-entry list is allowed. |
| Separators | Entries occupy separate logical lines, separated by commas; a final comma is optional. |
| Nested expressions | A nested expression consumes its own `with` and `end` delimiters before the enclosing delimiter. |
| Handler entry | A statically resolved handler path and context arguments, as in the existing single-handler form. |
| Block locals | Bindings in the body are unavailable in handler arguments and outside the handled body. |
| Invalid forms | Reject an empty list, missing separator, missing handler argument parentheses, or missing closing `end`. A block body requires the list form after `with`. |
| Contextual names | Preserve `handle::member()` as a qualified call path. |

There is no new comma-separated inline form. For example,
`handle body() with first(), second()` is rejected. The newline after `with`
and the final `end` make list boundaries explicit inside calls, records, and
other expressions that already use commas.

## Canonical formatting and automatic migration

`veln fmt` automatically rewrites eligible existing nested handles into the
canonical form below. This is part of ordinary formatting, with no migration
flag or semantic-analysis requirement. Existing spellings remain accepted
input. The current [format command](../specification/command-fmt.md) owns the
parse-before-write gate; this proposal retains that gate and its write policy.

The canonical handler sequence follows nesting from outermost to innermost.
A directly handled expression that is itself a handle contributes its entries
after the enclosing entries. This applies to both old single-handler syntax
and new list syntax, including mixtures. Parentheses that only group that
direct body do not prevent flattening. Entries are never sorted, deduplicated,
or selected by effect identity.

| Input shape after eligible flattening | Canonical output |
| --- | --- |
| One handler and a body that formats on one line | `handle Expr with Handler(arguments)`, without a closing `end`. A one-entry list converges to this spelling. |
| Multiple handlers and a body that formats on one line | Expression-body list form: `handle Expr with`, then one indented entry per line, then `end`. |
| A body that is exactly `begin Body end` | Block-body list form, even with one handler. Replace that one `begin` boundary with the equivalent handled block body; preserve its bindings and cleanup region. |
| An existing handled block body | Retain block-body list form, including a block with only one expression. Its body boundary remains explicit. |
| Any other multiline expression body | Block-body list form containing that expression as its body. Preserve the expression's own delimiters and cleanup regions. |
| Comments that require a list header or entry line | Retain list form even for one handler. Preserve those comment attachments. |

The expression-body form remains on one header line regardless of its length;
this proposal adds no line-width heuristic. Multiline handler arguments use
the existing argument layout. Every list entry starts one tab deeper than
`handle`, commas separate entries, and the last entry has no trailing comma.
For block-body form, `with` and the final `end` align with `handle`; body lines
use one additional tab. An explicit body boundary is retained even when its
body is empty or contains only one expression; formatting does not remove a
cleanup region merely because the current statements appear simple.

For example, both parenthesized and unparenthesized versions of this old chain:

```veln
handle (handle body() with inner(b())) with outer(a())
```

format as:

```veln
handle body() with
	outer(a()),
	inner(b())
end
```

Likewise, this input:

```veln
handle begin
	let input = read_input()
	process(input)
end with input_handler()
```

formats as:

```veln
handle
	let input = read_input()
	process(input)
with
	input_handler()
end
```

Flattening follows only the direct handled-expression chain. It does not
cross a `begin` body, statement, `defer`, function call, branch, or postfix
operator. In particular, `handle (handle body() with inner())? with outer()`
keeps the inner handle inside the `?` operand. A `begin` containing only an
inner handle still marks a body boundary: converting the enclosing `begin`
to block-body syntax does not merge that inner handle into the outer list.
An explicit `begin` nested inside another expression retains its own region.

Comments attached to handler applications move with their corresponding
entries. Body comments remain in the body. A comment attached to a delimiter
or grouping boundary that would disappear prevents that flattening or syntax
conversion unless its attachment can be preserved in the resulting syntax.
When a rewrite cannot preserve comment attachment, format that region with
its nesting intact; continue canonicalizing eligible regions elsewhere.
This is a defined boundary of canonicalization, not permission to discard
comments or guess a new attachment.

Formatting must preserve the ordered nesting contract, including argument
evaluation count and scope, body result, effect inference, and cleanup timing.
Formatting the output again must produce identical bytes. For comment-free
eligible inputs with the same direct nesting and body, legacy nesting and
list spelling must produce identical canonical output. The eight-prefix
motivating example must be rewritten automatically to the list shown above.

## Ordered nesting contract

For the purposes of observable semantics, a list is equivalent to nested
single-handler expressions with the first entry outermost:

```veln
handle body() with
	outer(a()),
	inner(b())
end
```

is equivalent to:

```veln
handle (handle body() with inner(b())) with outer(a())
```

For a block body, substitute `begin Body end` for `body()` in that model.
Equivalence includes result type and value, effect checking, context evaluation,
handler dispatch, task inheritance, resource-provider capture, and unwinding.
It does not require an implementation to construct nested syntax nodes.

| Observation | Required behavior |
| --- | --- |
| Context evaluation | Evaluate `a()` once before `b()`, then the body once. Arguments within each entry evaluate left to right. |
| Context scope | Evaluate an entry's arguments under the preceding outer entries and the surrounding scope. Its own handler and later entries are not active for those arguments. |
| Body dispatch | Every entry is active during the body. For repeated nominal effects, the last entry shadows earlier entries. Duplicate effects are allowed. |
| Handler dependencies | An inner handler's retained effects can be handled by preceding outer entries. Later inner entries do not discharge effects retained by an outer handler. |
| Context failure | If evaluation of `b()` fails or propagates `Err` with `?`, do not evaluate the body or subsequent entries. Restore all already entered frames with ordinary nested-handler cleanup. |
| Body exit | Normal completion, `?`, contract failure, runtime failure, and task cancellation follow existing nested-handler restoration and failure-precedence rules. |
| Deferred cleanup | Body cleanup runs while the list's handlers are active. Frames then unwind from the last entry to the first. Outer deferred cleanup observes the restored surrounding handlers. |
| Child tasks and resources | Child tasks inherit active handlers at creation. Captured provider identity and resource lifetimes remain those of the equivalent nesting, including after the list scope exits. |

Effect inference uses the same ordered model. Let entry `i` handle nominal
effect `E_i`, with context-argument effects `A_i` and retained declaration
effects `D_i`. Let `R_(n+1)` be the body effects, including deferred cleanup.
The remaining effects are `R_1`, where:

```text
R_i = A_i union (R_(i+1) minus {E_i}) union D_i
```

This rule deliberately does not subtract all handled effects from a single
union. An entry cannot discharge its own argument effects or effects retained
by an earlier outer entry. Public effect declarations and runnable-entry
rejection use `R_1` through the existing effect rules.

Each invalid handler path, argument, or operation retains the existing
single-handler validation rules. Human diagnostics identify the specific
failed fact at the relevant entry or argument span. Related notes carry
provenance and repair context. Desugaring must retain those source locations.

## Related work and alternatives

The inspected primary source is Daan Leijen's
[The Koka Programming Language, section 3.1.5](https://koka-lang.github.io/koka/doc/book.html#sec-with).
Koka's `with` statement passes the rest of a lexical scope as a function
argument and can flatten nested handler use. This supports exploring a flat
surface, but it is a broader abstraction than this Veln proposal. Veln keeps
an explicit handled body and a list restricted to named handlers; it does
not introduce trailing lambdas or general scope-passing statements.

Koka's
[Combining Handlers, section 3.4.6](https://koka-lang.github.io/koka/doc/book.html#sec-combine)
shows that changing handler order changes the combination of state and
exception results. This motivates an ordered model rather than an unordered
handler set. Koka's result-transforming handlers differ from Veln's
operation-result-preserving handlers. The exact Veln ordering and preservation
rules come from the existing Veln nesting contract, not from Koka.

Other alternatives have the following consequences:

| Alternative | Decision and reason |
| --- | --- |
| Reformat existing nesting | Does not remove repeated prefixes or scope counting; use the list surface instead. |
| Put the first entry innermost | Mirrors the current left-to-right `with` clause order, but reverses argument evaluation order relative to the list. Prefer outermost first so evaluation follows the written list. |
| Add named handler bundles | Defer bundle declarations, parameters, and composition rules until repeated list use establishes a separate abstraction need. |
| Let one handler implement several effects | Retain one nominal effect per handler; independent handlers can already compose under the proposed surface. |
| Merge the IO effects or change the registered view | Does not address composition of independent effects elsewhere. Keep IO capability/API redesign outside this syntax proposal. |

## Acceptance model and delivery

The nesting model above is the independent oracle for planned implementation
checks. Compare both spellings against explicit expected observations, so
two paths with the same defect do not count as sufficient evidence.

| Case | Required observation | Planned evidence |
| --- | --- | --- |
| Source forms | Accept both new forms, one and multiple entries, final comma, nested blocks, and use as a call argument or record field. Keep existing single-handler parsing. | Executable source grammar and accepted fixtures; syntax parser tests. |
| Invalid syntax | Reject every invalid form in the source table without consuming the next enclosing item during recovery. | Rejected grammar fixtures and parser recovery tests. |
| Canonical convergence | Legacy nesting, parenthesized direct nesting, mixed single/list nesting, and an already flat list produce the same outermost-first output. A one-entry list with a simple body becomes the short single-handler form; `handle begin` becomes the handled block form. | Formatter golden tests and checked CLI before/after formatting cases, including the eight-prefix motivating input. |
| Formatting stability | Preserve entry order, duplicate effects, argument expressions, and comments; remove the final comma. A second format produces identical bytes. | Formatter tests and checked CLI idempotence cases. |
| Migration boundaries | Preserve nesting across `?`, `begin` body statements, deferred cleanup, calls, and branches. Preserve delimiter comments or retain the affected nesting. Parse failure in any selected file leaves all files unchanged. | Formatter boundary tests and checked multi-file CLI cases. |
| Migration semantics | Old source and its actual formatter output have the same argument trace, shadowing, retained effects, result, and failure/cleanup trace. Include the partial-entry failure and provider-capture scenarios below. | Backend and semantic checks using formatter-produced source against explicit expected observations. |
| Independent effects | Two handlers remove their respective body effects and preserve the body result type and value. | Semantic tests and a compiler/backend integration example. |
| Shadowing | With two handlers for `Ask`, returning `1` and `2`, the body observes `2`; code after the list observes its preceding handler. | Scoped Veln fake support and backend dispatch checks. |
| Context scope and evaluation | Record outer arguments, inner arguments, and body in that order, exactly once. An inner argument performing `Ask` observes the outer entry, not itself or a later entry. | Semantic effect assertions and backend trace assertions. |
| Dependency direction | An outer `Ask` handler discharges `Ask` retained by an inner handler. Reversing the entries retains `Ask`; a public boundary must declare it. An outer argument's `Ask` remains outside that entry. | Semantic tests and checked diagnostic examples with entry spans and human related notes. |
| Partial entry and exit | Fail a later argument; fail and propagate from the body; verify later work is absent, body cleanup sees active handlers, and surrounding handlers are restored. Include cleanup-failure precedence. | Backend effect-injection and deferred-cleanup tests. |
| Tasks and provider capture | A child inherits both entries; a provider-bound resource keeps the creating provider after scope exit; cancellation restores enclosing frames. | Backend effect-injection and provider-boundary tests. |
| Motivating migration | Running the formatter replaces the eight-prefix expressions in the direct-validation helper with lists in reversed clause order. Keep existing operation, outcome, and snapshot assertions passing. | Formatter before/after evidence and existing stdlib companion tests for the direct IO facade. |

New CLI examples under `examples/specification/` target syntax, formatting,
compiler diagnostics, or backend integration. They do not replace stdlib API
coverage. IO behavior remains in `crates/veln-stdlib/veln/` companion tests under
the [test placement policy](../reference/toolchain-test-harness.md).
Deterministic tests use separate scoped Veln fake support, with real host
effects remaining the production default.

Implementation includes the executable grammar, parsing and recovery,
formatting, effect checking, source spans, lowering, and affected editor and
MCP source presentation. Reusing existing nested core handling is an
implementation option. The complete ordered contract must hold regardless of
representation. No handler values, result transformation, general `with`
statement, handler bundles, or IO API redesign are part of this delivery.

Before completion, run the relevant syntax, semantic, backend effect-boundary,
effect-injection, stdlib, and checked-example suites through the repository's
guarded runners. Promote implemented syntax and behavior to the source-surface,
effects, execution, and format-command specifications with executable evidence,
then remove
this proposal and its catalog entry. This documentation-only proposal does
not update current specifications or executable cases because the new forms
are not implemented yet.
