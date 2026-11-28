---
role: proposal
update-when: Automatic migration of nested handler expressions, comment-preserving flattening, or the remaining ordered-handler acceptance matrix changes.
---

# Multiple-handler Migration and Acceptance

The current [source-surface specification](../specification/source-surface.md)
defines the implemented terminated expression-body and block-body handler
lists. The [effects specification](../specification/effects.md) defines their
outermost-first ordered semantics, and the
[format specification](../specification/command-fmt.md) defines canonical
formatting for directly written lists and the implemented short, mixed-chain,
direct-`begin`, and existing-block migration.

This proposal now covers only the remaining migration forms and
ordered-contract cases that are not yet accepted by focused executable
evidence.

## Automatic migration

The current [format specification](../specification/command-fmt.md) defines the
implemented migration for comment-free single-line and eligible multiline
short chains and for eligible mixtures of short and terminated
expression-body forms. It also defines migration of a direct terminal `begin`
and of outer expression-bodied handlers around an existing handled block.
Remaining migration should support other multiline expression bodies and
comment forms without crossing a statement, `defer`, function call, branch, or
postfix operator.

The migrated output should use the short single-handler form when one handler
and a one-line body remain. Multiple handlers with a one-line body should use
the terminated expression form. Any remaining multiline expression body should
become the body of a handled block without removing its own delimiters.
Comments that need the list header or an entry line should keep a terminated
list even when only one handler remains.

Parentheses that only group the direct handled body may disappear. A comment
attached to a grouping or delimiter that would disappear prevents that local
rewrite unless the formatter can preserve the attachment. Body comments and
handler-entry comments must move with their source subjects. Formatting must
not discard or guess comment ownership.

A migrated chain must preserve entry order, context evaluation count and
scope, result value, effect inference, dispatch, cleanup timing, and provider
identity. Formatting the result again must produce identical bytes. The
formatter must preserve comment ownership while extending migration to the
remaining eligible forms.

## Remaining ordered-contract acceptance

The existing nested single-handler behavior remains the independent oracle.
The following cases still need focused acceptance before this proposal is
complete:

- retained-effect dependency direction, including reversal and outer argument
  effects;
- later-entry context failure and `?` propagation with already-entered frame
  restoration;
- cleanup-failure precedence across the complete list;
- child-task inheritance and cancellation restoration;
- provider-bound resource capture that outlives the list scope;
- the motivating direct-IO chain rewritten automatically without changing its
  companion-test outcomes.

These checks must use scoped Veln fake handlers where deterministic effects
are required. Production handlers continue to use real host effects.

## Acceptance evidence

Implementation is complete when checked formatter cases cover migration of the
remaining single-entry, multiline-expression, and comment-preserving inputs,
and semantic/backend cases cover the remaining contract above. The current
parse-before-write gate must continue to leave every selected file unchanged
when any file has a parse diagnostic.

After those checks pass, remove this proposal and its Ready catalog entry. The
current specifications and executable examples must remain the authority for
the implemented behavior.

## Related work

Daan Leijen's
[Koka `with` statement](https://koka-lang.github.io/koka/doc/book.html#sec-with)
can flatten nested handler use by passing the remaining lexical scope. It
supports a flat presentation, but it is broader than Veln's named handler list.
Veln keeps an explicit handled body and does not add a general scope-passing
statement.

Koka's
[combining-handlers discussion](https://koka-lang.github.io/koka/doc/book.html#sec-combine)
shows that handler order can change composed results. That finding supports
preserving written order during migration. Veln's exact ordering and
result-preserving rules come from its existing nested-handler contract.
