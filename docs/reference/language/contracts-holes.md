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
- `result` in `ensure` clauses

Contract predicates containing `stdio::`, call-like syntax, empty predicates,
non-boolean predicates, or unresolved names produce diagnostics. Valid
contracts are recorded and may contribute hole repair constraints, but runtime
contract enforcement is not implemented.

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
