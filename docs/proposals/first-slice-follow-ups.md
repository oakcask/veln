# First-Slice Follow-Up Targets

Status: accepted-proposal
Implementation: partially implemented

This document tracks accepted first-slice targets that are not fully
implemented in the current workspace. The completed implementation sequence
stays in
[../phases/first-slice-implementation.md](../phases/first-slice-implementation.md).

## Language And Type Coverage

No accepted language and type coverage follow-up is currently tracked here.

## Repair Loop

- `hole.unfilled` emits candidate-query records when an expected type is
  known and ranks visible assignable symbol candidates when available.
- `satisfy` suffix parsing, formatting, constraint exposure, missing candidate
  diagnostics, missing `=>` diagnostics, candidate shadowing diagnostics, and
  unused candidate diagnostics are implemented. Satisfy predicates are
  semantically validated against the pure boolean predicate subset with the
  candidate bound to the hole expected type when known. Direct equality
  and direct inclusive comparison satisfy-constrained symbol repair candidates
  are generated as unapplied safe repair candidates when the predicate becomes
  reflexive for the same visible binding. Tautological equality and inclusive
  comparison predicates on the satisfy candidate itself mark every
  type-compatible visible binding candidate as an unapplied safe repair
  candidate, including parenthesized direct and tautological clauses. Direct
  top-level `or` branches recognize each reflexive branch independently, so
  each visible binding named by a reflexive branch can become an unapplied safe
  repair candidate. Satisfy predicates also recognize direct field-access
  reflexive clauses where the candidate and visible binding share the same
  field suffix. Satisfy predicates may include
  literal `true` conjuncts without changing direct, tautological, or
  `require`-matched repair status. Satisfy predicates whose
  candidate substitution is already guaranteed by a valid `require` clause mark
  the matching visible binding as an unapplied safe repair candidate, including
  string-literal clauses and simple direct, commuted, parenthesized comparison
  clauses, and whole parenthesized `and` conjunctions. Negated equality and
  disequality clauses normalize into their inverse comparisons before matching.
  Negated direct `satisfy` equality and ordering clauses also normalize before
  direct reflexive and tautological repair matching.
  Strict ordering `require` clauses also discharge the corresponding inclusive
  ordering and disequality `satisfy` clauses for the same operands. Equality
  `require` clauses discharge inclusive ordering `satisfy` clauses for the same
  operands in either direction. Paired inclusive bounds from valid `require`
  clauses discharge equality `satisfy` clauses for the same operands. Negated
  ordering clauses normalize into their inverse comparisons before matching
  against valid `require` clauses. They also accept top-level `or` predicates
  when at least one branch is fully guaranteed by valid `require` clauses, and
  top-level `or` in `require` predicates when every branch guarantees the
  substituted `satisfy` clause. Nested `or` branches inside `and` conjunctions
  are also recognized when every branch guarantees the same substituted clause.
  Nested `or` branches inside `satisfy` `and` conjunctions are recognized when
  at least one branch is guaranteed by valid `require` clauses. Literal
  `false` disjuncts do not affect direct, tautological, or `require`-matched
  repair status. Top-level literal `true` disjuncts make a `satisfy` predicate
  tautological for repair ranking. Top-level disjuncts that are tautological
  for the satisfy candidate also make the whole `satisfy` predicate
  tautological for repair ranking.
  Same-shape direct expression equality and inclusive comparison clauses are
  accepted when replacing the satisfy candidate with one visible binding makes
  both sides textually identical after whitespace normalization.
  Transitive inclusive ordering paths in both directions discharge equality
  `satisfy` clauses for the endpoints. Negated top-level `or` predicates in
  valid `require` clauses discharge direct comparison `satisfy` clauses through
  their inverted branches. Nested literal `true` disjuncts inside direct
  `satisfy` conjunctions and `require`-matched `satisfy` conjunctions are
  treated as tautological surplus clauses for repair ranking. Negated
  disjunctions of direct `satisfy` comparison clauses are normalized into their
  inverted comparison clauses before direct repair matching. Negated
  conjunctions of direct `satisfy` comparison clauses are normalized into
  disjunctive direct branches before direct repair matching and before
  `require`-matched repair discharge. Negated conjunctions in valid `require`
  clauses discharge top-level disjunctive `satisfy` predicates when every
  inverted branch guarantees one satisfy branch. Disjunctive
  `require` clauses contribute transitive ordering evidence when every branch
  guarantees the same weaker comparison, such as treating
  `require low < mid or low == mid` as an inclusive `low <= mid` edge for
  repair discharge. Literal boolean negation is normalized during repair
  matching, so `not false` participates as a tautological clause and
  `not true` participates as a false disjunct. Double negation around direct
  comparison clauses is normalized before direct and `require`-matched repair
  ranking. Nested literal `true` disjuncts inside tautological `satisfy`
  conjunctions are treated as tautological surplus clauses for repair ranking.
  Complementary top-level disjuncts over the same candidate-referencing
  predicate, such as `candidate.ready or not candidate.ready`, also make the
  whole `satisfy` predicate tautological for repair ranking.
  Same-shape expression tautologies rooted at the satisfy candidate are
  accepted after whitespace normalization.
  Negated top-level `and` predicates with complementary
  candidate-referencing branches are treated as tautological for repair
  ranking.
  Contract obligation classification treats complementary boolean identities
  across more than two top-level `or` branches, and the negation of more than
  two top-level `and` branches containing a complementary pair, as statically
  proven after validation.
  Negated disjunctions with literal `false` branches are normalized for direct
  `satisfy` repair matching and for `require`-matched repair discharge.
  Literal boolean branches created while normalizing negated conjunctions are
  folded for direct `satisfy` repair matching and for `require`-matched repair
  discharge.
  Negated disjunctions in valid `require` clauses expose negated boolean atom
  branches for `require`-matched repair discharge.
  Disjunctive `require` clauses with a common boolean atom across every branch
  also expose that atom for `require`-matched repair discharge.
  Same-shape expression operands in `require`-matched repair comparisons are
  compared after whitespace normalization, including equality aliases inside
  those expression operands. Equality aliases also apply while chaining
  transitive ordering evidence over same-shape expression operands. Equality
  aliases also discharge boolean atom `satisfy` clauses when the aliased atom
  is already guaranteed by a valid `require` clause. Boolean atom and literal
  boolean equality or disequality clauses also discharge each other during
  `require`-matched repair.
  Top-level disjunctive `require` predicates discharge top-level disjunctive
  `satisfy` predicates when every `require` branch guarantees at least one
  `satisfy` branch.
  Disjunctive equality requirements against distinct boolean, integer, or
  string literals discharge disequality `satisfy` clauses against another
  literal, including through equality aliases. Inclusive transitive ordering
  plus endpoint disequality discharges strict comparison `satisfy` clauses.
  Stronger numeric literal bounds discharge weaker numeric literal bounds over
  the same subject using exact decimal literal ordering, while equal inclusive
  bounds do not discharge strict bounds. Numeric literal bounds also discharge
  disequality `satisfy` clauses against numeric literals excluded by the bound,
  while equal inclusive bounds do not discharge disequality against the
  endpoint. Numeric literal equality requirements discharge weaker numeric
  literal bound `satisfy` clauses over the same subject. Statically satisfied
  repair candidates are retained even when they sort after the ordinary
  manual-review candidate bound.
  Nested complementary `or` clauses rooted at the satisfy candidate are
  treated as tautological surplus clauses inside direct and tautological
  `satisfy` conjunctions.
  Broader repair discharge beyond these normalized direct and
  `require`-matched cases remains follow-up work before formatter
  stabilization.

## Effects And Contracts

- Direct stdio calls are recognized as compiler-known effectful prelude calls,
  private helper body effects propagate to callers, and effect diagnostics
  expose bounded path entries with hidden-frame and omitted-path counts.
- The executable bounded-channel slice is implemented and specified in the
  language reference. The executable task slice now covers `spawn`, task
  handles, cancellation, and join. Test-visible stdio event capture now
  serializes each output operation with its event sequence. The executable
  two-receiver channel selection slice is implemented, including a timeout
  variant, rotating ready-receiver tie breaking, explicit left-priority
  selection, and result-returning selection variants that report cooperative
  cancellation separately from closed or timed-out selection.
- The checker validates the first-slice pure boolean contract subset. Literal
  boolean contract predicates that evaluate to `true` are statically discharged.
  Runtime contract discharge is implemented for function-entry `require` checks
  and `ensure` checks before both ordinary returns and `?` early returns.
  `veln test --json` reports runtime contract failures inside selected test
  cases as structured failed-case details, and `veln run --json` reports
  runtime contract failures as top-level structured errors.
- Contract predicates now parse through a dedicated first-slice predicate
  production. Bare and `use`-alias qualified pure calls to discovered
  effect-free functions are validated and participate in selected-entry
  reachability for executable commands. The implemented subset now treats
  string-literal contents as literal text during predicate name, call, and field
  discovery, accepts string equality and disequality comparisons with the
  literal on either side, and validates pure prelude helper calls inside
  contract and `satisfy` predicates. Contract obligation classification also
  statically proves boolean identity cases where one side of `or` is already
  true, literal-only comparisons that evaluate to true, and propagation of
  those truths through literal-only boolean wrappers. It also statically proves
  top-level complementary boolean disjunctions such as `flag or not flag` after
  validation, and negated top-level complementary boolean conjunctions such as
  `not (flag and not flag)`. The current implemented predicate subset is
  specified in the language reference. Richer predicate semantics beyond these
  static truth identities, literal comparisons, complementary boolean
  disjunctions, and negated complementary boolean conjunctions remain follow-up
  work.

## Formatting

No accepted formatting follow-up is currently tracked here.

## Lowering And Execution

No accepted lowering and execution follow-up is currently tracked here.

## Test Discovery And Events

No accepted test discovery follow-up is currently tracked here.
