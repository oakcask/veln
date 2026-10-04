---
role: proposal
update-when: Handler target syntax, the handles keyword, handler formatting, effect navigation, or the planned syntax evidence changes.
---

# Handler Targets With `for`

## Purpose And Scope

Replace `handler h() handles E` with `handler h() for E`. Reuse the existing
`for` keyword and remove `handles` from the keyword set. The target remains
one nominal effect. This proposal changes syntax and lexical classification;
it preserves the semantics in [Effects](../specification/effects.md) and
[Execution](../specification/execution.md).

The following is proposed syntax, not currently accepted syntax:

```veln
effect Ask
	value() -> Int
end

handler ask(offset: Int) for Ask
	value() => offset + 1
end

fn compute() -> Int effects [Ask]
	perform Ask::value()
end

fn answer() -> Int
	handle compute() with ask(41)
end
```

`answer()` returns `42` and retains no effect. A handler with other retained
effects can declare them separately, for example
`pub handler audit(ctx: Context) for telemetry::Audit effects [stdio]`.

## Proposed Contract

```text
HandlerDecl ::= "pub"? "handler" Name "(" ParamList? ")" "for" MemberPath Effects? NL HandlerOperationClause+ "end" NL?
```

The other productions keep their current meaning. `for` is required between
the closing parameter parenthesis and the effect path. `for` remains reserved;
it does not become a contextual identifier. `handles` becomes an ordinary
identifier wherever an ordinary identifier is accepted. Function names, local
bindings, parameters, fields, and module path segments named `handles` follow
the existing casing and name-resolution rules.

The former handler separator is rejected. There is no compatibility alias or
automatic migration requirement. Parser failures identify the missing or
invalid separator at the relevant span and report `for` as the expected token.

Effect resolution, visibility, exact-companion access, operation completeness,
clause typing, and retained-effect checks keep their current behavior. Runtime
argument evaluation order, deep handling, nested shadowing, task inheritance,
cleanup, and scope restoration also keep their current behavior.

## Acceptance Model

These cases describe planned evidence, not implemented or passing evidence.

| Input or observation | Required outcome | Planned verification |
| --- | --- | --- |
| Private or public `for` handler, with context parameters, a bare or qualified target, and an optional retained-effect list. | Accept otherwise valid declarations and resolve the same effects as before. | Accepted executable-grammar fixtures, syntax tests, and compiler check cases. |
| Handler header using the former separator, or omitting the separator. | Reject it with a separator diagnostic expecting `for`; recovery preserves the following declaration. | Rejected grammar fixtures and parser recovery tests; human and JSON diagnostic cases. |
| `handles` outside the obsolete separator position. | Lex as `Ident` and accept it under ordinary name rules. Public keywords exclude `handles` and retain `for`. | Lexer and public-token tests; declaration, binding, qualified-reference, and source-less lookup validation tests. |
| Formatting a valid `for` handler. | Emit `for`, preserve the effect path and retained-effect list, and produce a parseable, idempotent result. | Formatter round-trip cases and syntax tests. |
| Selecting the effect in a bare or qualified handler target. | Preserve definition and reference results, correct source ranges, and semantic token classification. | Language-service navigation and editor semantic-token tests, including UTF-16 range coverage. |
| Published language reference and lexical editor fallback. | Grammar, keyword lists, examples, and highlighting agree with the proposed syntax and ordinary `handles` identifiers. | Language-reference catalog generation and freshness checks; VSCode grammar checks and editor tests. |
| Running the example above and existing nested-handler, early-return, cleanup, task, and injected-host-effect cases with migrated headers. | Preserve results, effect sets, operation order, scope restoration, and provider ownership. | Existing compiler, JVM, and CLI handler coverage; affected stdlib companion tests for stdlib behavior. |
| Invalid operation clause, inaccessible effect, or wrongly retained handled effect. | Preserve existing semantic rejection, with spans computed from the new source text. | Existing semantic diagnostic cases with migrated headers and expected ranges. |

## Related Work And Alternatives

The relevant comparison is Veln's existing declaration boundary, token model,
and handler representation. No cross-language semantic change is proposed.

- [Current grammar](../specification/source-surface.md) places a single target
  separator after the complete parameter list. The
  [handler parser](https://github.com/oakcask/veln/blob/bbf7d528e4235242dfa60b8291dfd961b8b08b4b/crates/veln-syntax/src/parser/declarations.rs) confirms
  that boundary. This supports replacing the separator without new lookahead
  or a changed target production. It does not prove the proposed syntax works;
  the acceptance cases must verify that inference.
- The [token model](https://github.com/oakcask/veln/blob/bbf7d528e4235242dfa60b8291dfd961b8b08b4b/crates/veln-syntax/src/token.rs) already includes
  `for`. The
  [expression parser](https://github.com/oakcask/veln/blob/bbf7d528e4235242dfa60b8291dfd961b8b08b4b/crates/veln-syntax/src/parser/expression_primaries.rs)
  accepts `handles` as a contextual name. Reusing `for` removes one keyword;
  ordinary identifier classification preserves existing name uses and allows
  ordinary binding uses. Keeping a contextual or reserved `handles` token
  would leave the vocabulary reduction incomplete.
- The [handler AST](https://github.com/oakcask/veln/blob/bbf7d528e4235242dfa60b8291dfd961b8b08b4b/crates/veln-syntax/src/ast.rs) records the effect path
  rather than the separator, and
  [semantic analysis](https://github.com/oakcask/veln/blob/bbf7d528e4235242dfa60b8291dfd961b8b08b4b/crates/veln-sema/src/analysis/handlers.rs) resolves
  that path. This supports preserving semantics, but does not prove editor
  equivalence. The
  [navigation classifier](https://github.com/oakcask/veln/blob/bbf7d528e4235242dfa60b8291dfd961b8b08b4b/crates/veln-language-service/src/navigation/token_roles.rs)
  and [editor header classifier](https://github.com/oakcask/veln/blob/bbf7d528e4235242dfa60b8291dfd961b8b08b4b/crates/veln-editor/src/semantic_tokens/classifier_collection.rs)
  explicitly recognize `Handles`; both need corresponding evidence.

Keeping `handles` makes the handling relationship more explicit but retains a
dedicated keyword for a position already identified by `handler`. Accepting
both spellings reduces migration work but preserves two forms and prevents
complete keyword removal. This proposal chooses one canonical `for` form;
source compatibility is not required by the experimental project.

## Implementation And Completion

Update the parser, formatter, token records and indexed token labels,
contextual-name handling, source-less keyword validation, editor classifiers,
and navigation recognition together. Migrate repository-owned handler headers
in source, test support, examples, and generated source carriers. Do not replace
unrelated identifier uses of `handles` with `for`.

Update the executable grammar and accepted and rejected fixtures, then the
current source-surface, effect, and editor specifications. Regenerate the
language-reference catalog from those authorities. Verification uses the
existing grammar, syntax, semantic, editor, language-service, and toolchain
harnesses; their owning specifications and package tests define the check
routes. Test placement follows
[Toolchain Test Harness](../reference/toolchain-test-harness.md).

Completion requires the acceptance model to pass, all active source surfaces
to agree, and review to confirm that no handler semantic change was introduced.
Then remove this proposal and its catalog entry. Until implementation, current
specifications and executable fixtures continue to describe `handles`.
