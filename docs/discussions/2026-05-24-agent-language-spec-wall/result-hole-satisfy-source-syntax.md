# Discussion Result: Hole Satisfy Source Syntax

Date: 2026-05-24

## Picked Question

- What is the exact source syntax for attaching `satisfy` predicates to holes
  and naming the hole candidate value?

## Decision

Attach a first-slice `satisfy` predicate directly to a hole expression with a
hole-only suffix:

```text
_port satisfy candidate => candidate > 0 and candidate <= 65535
_ satisfy value => value.name != ""
```

The first-slice grammar should extend `Hole` like this:

The consolidated production is maintained in
[Veln First-Slice Grammar](../../reference/grammar.md#holes).

`BindingName` names the candidate value that would replace the hole. The
binding is read-only and scoped only to the `ContractPredicate` after `=>`.
The named-hole label, such as `_port`, remains only a diagnostic and repair
label; it is not the candidate binding.

The first slice should allow at most one `satisfy` suffix per hole. Multiple
constraints should be written as one predicate with `and` until examples show
that repeated clauses are worth the parser and formatter surface area.

## Rationale

Typed-hole research supports treating a hole as a real partial-program
expression with type and local context. Hazelnut and Hazel both motivate
keeping incomplete expressions inside the ordinary checked structure instead
of moving repair facts into comments or side channels. A suffix on the hole
keeps the predicate source-backed and gives the parser, AST, formatter, and
diagnostic path one local node to track.

Type-directed completion frames holes and partial expressions as a search
interface. For Veln, the expected type supplies the coarse candidate shape,
while the `satisfy` predicate supplies a local semantic filter. The candidate
binding should therefore be explicit in source; relying on a magic name such
as `result`, or reusing the named-hole label as a binding, would blur the
difference between "where the repair goes" and "the value being tested".

Liquid Types is useful as a boundary case. Rich predicate-guided checking works
when the predicate language is restricted and solver-friendly. Veln's first
slice should not imply a full refinement-type system, so the suffix should
reuse `ContractPredicate` exactly and should attach only to holes. It is repair
guidance for one missing expression, not a general expression annotation
syntax.

The `satisfy candidate => predicate` spelling also keeps parser recovery
simple. `satisfy` can only appear immediately after `HoleAtom`, and `=>`
separates the binder from the predicate using punctuation already present in
the first-slice grammar for match arms. A malformed suffix can recover at the
next newline, comma, closing delimiter, contract clause, or `end` without
making every expression parse as an optionally constrained value.

## First-Slice Rules

- `satisfy` is valid only as a suffix on `_` or `_name` hole expressions.
- A `satisfy` suffix must include exactly one candidate binding before `=>`.
- The candidate binding must be a lower-name binding accepted by the ordinary
  pattern-binding lexer, excluding `_` and `_name`.
- The candidate binding is visible only inside the predicate after `=>`.
- The candidate binding should not duplicate a visible local, parameter,
  result binding, import alias, or prelude name. Report a targeted
  `hole.satisfy_candidate_shadow` diagnostic when this can be detected.
- The predicate must reference the candidate binding at least once. A
  candidate-free predicate is not a value constraint and should be reported as
  `hole.satisfy_candidate_unused`.
- The predicate uses the existing `ContractPredicate` grammar and semantic
  validation rules.
- A filled candidate expression must typecheck against the hole's expected
  type before the predicate is validated in the candidate context.
- `veln fmt` preserves the hole label, `satisfy` keyword, candidate binding,
  `=>`, and predicate. It may parenthesize the whole hole expression when
  needed inside larger expressions.
- Detached forms such as a later `satisfy _port ...` clause are not part of
  the first slice because named holes are labels, not bindings.

## Examples

```text
pub fn default_port(max: Int) -> Int effects []
require max > 0
  _port satisfy candidate => candidate > 0 and candidate <= max
end
```

```text
fn default_user() -> User
  _user satisfy value => value.name != "" and value.active == true
end
```

## Open Detail

The broader scoping and duplicate-name policy for ordinary `let` bindings,
imports, public API membership, result bindings, and named holes remains open.
This decision only gives the `satisfy` suffix a conservative local
non-shadowing rule so the first checker can resolve the candidate binding
without waiting for the full name-resolution decision.

Future syntax may add repeated `satisfy` clauses or a block form if examples
show that predicates routinely become too long for an expression suffix. That
would extend this decision rather than changing the first-slice meaning of
`satisfy candidate => predicate`.

## References

- Omar, C., Voysey, I., Hilton, M., Aldrich, J., & Hammer, M. A. (2017).
  Hazelnut: A bidirectionally typed structure editor calculus. *POPL 2017*,
  86-99. https://doi.org/10.1145/3009837.3009900
- Omar, C., Voysey, I., Chugh, R., & Hammer, M. A. (2019). Live functional
  programming with typed holes. *Proceedings of the ACM on Programming
  Languages*, 3(POPL), 1-32. https://doi.org/10.1145/3290327
- Perelman, D., Gulwani, S., Ball, T., & Grossman, D. (2012). Type-directed
  completion of partial expressions. *PLDI 2012*, 275-286.
  https://doi.org/10.1145/2254064.2254098
- Rondon, P. M., Kawaguchi, M., & Jhala, R. (2008). Liquid types.
  *PLDI 2008*, 159-169. https://doi.org/10.1145/1375581.1375602
- Medeiros, S. Q. de, Alvez Junior, G. de A., & Mascarenhas, F. (2019).
  *Automatic Syntax Error Reporting and Recovery in Parsing Expression
  Grammars*. arXiv:1905.02145. https://arxiv.org/abs/1905.02145

## Consequence

The first parser gets a concrete, local syntax for constrained holes without
turning named holes into bindings or adding a second specification language.
Agents can read one source-backed hole node, one explicit candidate binding,
and one predicate when ranking or validating candidate fills.
