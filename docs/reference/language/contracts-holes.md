# Contracts And Holes

This file specifies implemented contract predicate validation, holes, and
repair constraints.

## Contracts

Implemented contract clauses are `require` and `ensure` lines attached to a
function. The parser first checks a narrow contract predicate syntax. It
accepts literals, names, qualified names, grouping, field access syntax,
plain or qualified call syntax, prefix operators, arithmetic operators,
comparisons, equality, and boolean operators.

The parser rejects holes, `?`, pipelines, `match`, records, and lists in
contract predicates. Unsupported contract syntax in `require` or `ensure`
reports `parse.contract_predicate`; unsupported syntax in a hole `satisfy`
predicate reports `parse.satisfy_predicate`.

After parsing, the checker validates a small pure boolean subset:

- `true` and `false`
- boolean bindings visible to the clause
- `and`, `or`, and `not`
- comparison and equality operators
- field access on record-typed bindings visible to the clause
- calls to discovered pure functions when arguments are assignable to the
  declared parameter types and the return type fits the predicate position
- visible parameter bindings
- explicit result bindings in `ensure` clauses

Contract predicates containing `stdio::`, effectful function calls,
unsupported call targets, empty predicates, missing record fields, non-boolean
predicates, or unresolved names produce diagnostics. Valid contracts are
recorded and may contribute hole repair constraints, but runtime contract
enforcement is not implemented.

An `ensure` clause may refer to the returned value only when the function return
position names it with `-> name: Type`. That name is not visible to `require`
clauses or the function body. The identifier `result` is ordinary: without an
explicit binding named `result`, it reports an unresolved-name diagnostic.

## Holes

Holes produce `hole.unfilled` diagnostics with severity `hint`. A check result
with only non-error hole diagnostics has top-level status `partial`.

Hole details include:

- `phase`
- `node_id`
- `label`
- `expected_type`
- `expected_type_source`
- `constraints`
- `local_bindings`
- `candidate_queries`

Candidate query records are advisory only. Each query carries
`candidate_status: "query_only"` and
`application_policy: "manual_review_required"` to make clear that the checker
has not produced or authorized an edit.

When a hole has a known expected type, the symbol query may include ranked
visible binding candidates. A ranked candidate is not an edit. It records a
binding name, rendered binding type, rank, match reason, and the same manual
review application policy. Exact type matches rank before broader assignable
matches, and nearer visible bindings rank before older bindings with the same
match quality.

Named holes such as `_port` are diagnostic and repair labels, not bindings.
The `satisfy candidate => predicate` suffix contributes a repair constraint; it
does not bind `candidate` outside the suffix predicate.

`satisfy` suffixes must include one candidate binding and `=>`. Missing
candidate bindings report `parse.satisfy_candidate`; missing arrows report
`parse.satisfy_arrow`.

The candidate binding is scoped only to the suffix predicate. It must not
shadow visible local bindings, parameters, explicit result bindings, or
compiler-known prelude helper names. Shadowing reports
`hole.satisfy_candidate_shadow`.

The predicate must reference the candidate binding at least once. A predicate
that omits the candidate reports `hole.satisfy_candidate_unused`.
