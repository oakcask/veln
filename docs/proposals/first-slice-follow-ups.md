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
  ranking, including parenthesized nested `and` branches.
  Contract obligation classification treats complementary boolean identities
  across more than two top-level or parenthesized nested `or` branches, and the
  negation of more than two top-level or parenthesized nested `and` branches
  containing a complementary pair, as statically proven after validation.
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
  Disjunctive alias requirements are also checked branch by branch against the
  other valid `require` clauses, so a disjunction that equates the substituted
  candidate with one of several bindings can discharge a `satisfy` clause when
  every branch has matching evidence.
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
  literal bound `satisfy` clauses over the same subject. Numeric literal bounds
  may include pure literal arithmetic subexpressions. Numeric disequality
  requirements discharge strict ordering disjunctions around the excluded
  literal, including through equality aliases. Statically satisfied repair
  candidates are retained even when they sort after the ordinary manual-review
  candidate bound.
  Nested complementary `or` clauses rooted at the satisfy candidate are
  treated as tautological surplus clauses inside direct and tautological
  `satisfy` conjunctions.
  Top-level complementary comparison disjuncts that reference the satisfy
  candidate are also treated as tautological repair constraints after
  whitespace normalization and commuted ordering normalization.
  Top-level ordering trichotomy disjuncts that reference the satisfy candidate
  are also treated as tautological repair constraints after whitespace
  normalization and commuted ordering normalization.
  Top-level inclusive ordering totality disjuncts that reference the satisfy
  candidate are also treated as tautological repair constraints after
  whitespace normalization and commuted ordering normalization.
  Negated top-level `and` predicates with mutually exclusive ordering
  trichotomy clauses rooted at the satisfy candidate are also treated as
  tautological repair constraints. Negated top-level `and` predicates with
  opposite inclusive and strict ordering bounds rooted at the satisfy
  candidate are also treated as tautological repair constraints.
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
  `not (flag and not flag)`. Complementary comparison pairs such as
  `value == limit or value != limit` and
  `not (value < limit and value >= limit)` are also statically proven after
  whitespace normalization and commuted ordering normalization. The current
  implemented predicate subset is specified in the language reference.
  Same-shape comparison predicates are also statically evaluated after
  whitespace normalization. Literal numeric `+`, `-`, `*`, and exactly
  representable `/` subexpressions inside comparisons are also statically
  evaluated. Balanced grouping around literal comparison operands does not
  prevent static literal comparison. Boolean equality and disequality over
  statically known boolean subexpressions are also statically evaluated.
  Equality and disequality comparisons between complementary pure predicates
  are also statically evaluated after validation.
  Top-level ordering trichotomy disjunctions over the same operands, and
  negated conjunctions containing mutually exclusive ordering trichotomy
  relations over the same operands, are also statically evaluated after
  whitespace normalization and
  commuted ordering normalization. Inclusive ordering totality disjunctions
  over the same operands, and negated conjunctions with opposite inclusive and
  strict ordering bounds over the same operands, are also statically evaluated
  after whitespace normalization and commuted ordering normalization.
  Negated conjunctions with one disjunction branch whose non-static disjuncts
  are all covered by complement conjuncts are also statically evaluated.
  Case-split top-level `or` predicates whose complemented branch only adds
  statically true conjuncts are also statically evaluated, including boolean
  atoms and direct comparison complements. Top-level `or` predicates whose
  repeated branch appears inside a negated `and` conjunction are also
  statically evaluated. Top-level `or` predicates with one conjunction branch
  whose non-static conjuncts are all covered by complement disjuncts are also
  statically evaluated. Factored case-split top-level `or` predicates are also
  statically evaluated when two conjunction branches differ by one
  complementary predicate and the remaining shared predicates are covered by
  complement disjuncts. Case-split top-level `or` predicates with shorter
  branches that cover the remaining assignments for the same predicate set are
  also statically evaluated. Case-split top-level `or`
  predicates where both branches are conjunctions with one complementary
  non-static variant and otherwise statically true conjuncts are also
  statically evaluated. Exhaustive pair case splits that cover both polarities
  of two non-static predicates across four top-level conjunction branches are
  also statically evaluated. Exhaustive triple case splits that cover both
  polarities of three non-static predicates across eight top-level conjunction
  branches are also statically evaluated. Exhaustive quad case splits that
  cover both polarities of four non-static predicates across sixteen top-level
  conjunction branches are also statically evaluated. Exhaustive quint case
  splits that cover both polarities of five non-static predicates across
  thirty-two top-level conjunction branches are also statically evaluated.
  Exhaustive sext case splits that cover both polarities of six non-static
  predicates across sixty-four top-level conjunction branches are also
  statically evaluated. Exhaustive sept case splits that cover both polarities
  of seven non-static predicates across one hundred twenty-eight top-level
  conjunction branches are also statically evaluated. Exhaustive oct case
  splits that cover both polarities of eight non-static predicates across two
  hundred fifty-six top-level conjunction branches are also statically
  evaluated.
  Negated conjunctions where a nested disjunction repeats one outer conjunct,
  such as `not (flag and not (flag or ready))`, are also statically evaluated.
  Resolved complementary disjunctions contradicted by another conjunct, such
  as `not (flag and (not flag or ready) and (not flag or not ready))`, are also
  statically evaluated.
  Boolean literal aliases such as `flag == true`, `false == flag`, and
  `flag != false` also participate in complementary static truth identities.
  Richer predicate semantics beyond these static truth identities, literal
  comparisons, literal numeric arithmetic comparisons, same-shape comparisons,
  static boolean comparisons, complementary predicate comparisons,
  complementary boolean and comparison disjunctions, ordering trichotomy
  disjunctions, negated complementary boolean and comparison conjunctions, and
  negated mutually exclusive order conjunctions remain follow-up work.

## Formatting

No accepted formatting follow-up is currently tracked here.

## Lowering And Execution

No accepted lowering and execution follow-up is currently tracked here.

## Test Discovery And Events

No accepted test discovery follow-up is currently tracked here.
