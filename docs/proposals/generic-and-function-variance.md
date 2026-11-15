---
role: proposal
update-when: Generic variance whole-analysis or language-service failure preservation changes.
---

# Generic And Function Variance

## Summary

This proposal contains the remaining failure-preservation work for named
generic variance. It does not add declaration-site variance modifiers.

## Remaining Failure Preservation

Rejected analysis or assignment beyond a local join contribution must publish
applicable diagnostics without executing the rejected program or exposing a
partial successful signature. An applicable failure must preserve any retained
successful language-service snapshot under its existing failure rules.

The evidence below is planned, not implemented or passing. The behavior above
is its independent acceptance oracle.

Whole-analysis and language-service regression cases must prove that no partial
signature escapes rejection, retained successful snapshots survive applicable
failures, and rejected programs do not run.

Compiler evidence belongs in the existing semantic test suites and
`examples/specification/check/` with diagnostic companions. Run the existing
executable specification harness and focused package, LSP, and MCP suites for
their respective contracts.

Once those checks pass, update the matching current specifications and
executable examples. Then retire this proposal and its catalog entry.

## Scope And Implementation Basis

This proposal is independently ready. It does not depend on the remaining
refinement-flow, repair, or handler proposals.
It excludes mutable references, declaration-site variance syntax, general
unions/intersections, higher-kinded types, predicate-based subtyping, and
runtime representation changes solely for variance.
