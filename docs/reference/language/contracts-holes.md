# Contracts And Holes

This file specifies implemented contract predicate validation, holes, and
repair constraints.

## Contracts

Implemented contract clauses are `require` and `ensure` lines attached to a
function. The checker validates a small pure boolean subset:

- `true` and `false`
- boolean local bindings
- `and`, `or`, and `not`
- comparison and equality operators
- referenced parameters and local bindings
- explicit result bindings in `ensure` clauses

Contract predicates containing `stdio::`, call-like syntax, empty predicates,
non-boolean predicates, or unresolved names produce diagnostics. Valid
contracts are recorded and may contribute hole repair constraints, but runtime
contract enforcement is not implemented.

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
