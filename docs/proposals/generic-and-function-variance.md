---
role: proposal
update-when: Dependency variance metadata, variance presentation, or analysis and language-service failure preservation change.
---

# Generic And Function Variance

## Summary

This proposal contains the remaining work for named generic
variance. The remaining work adds dependency invalidation, stable formatter,
package, LSP, and MCP presentation, and whole-analysis and
language-service failure preservation. This proposal does not add
declaration-site variance modifiers.

## Remaining Invalidation, Presentation, And Failure

| Surface | Selected behavior |
| --- | --- |
| Declaration changes | Recompute affected variance contracts and invalidate dependent analysis before accepting downstream assignments. A stale classification cannot survive a payload change. |
| Formatter, package, LSP, MCP | Keep ordinary type presentation stable across these surfaces without exposing inferred variance modifiers or replacement type names. |
| Rejected analysis or assignment beyond a local join contribution | Publish applicable diagnostics without executing the rejected program or exposing a partial successful signature. Preserve any retained successful language-service snapshot under its existing failure rules. |

The remaining evidence below is planned, not implemented or passing. The
remaining-behavior table above is its independent acceptance oracle.

| Contract | Planned evidence and required observation |
| --- | --- |
| Metadata invalidation | Package fixtures prove that a variance-changing payload edit invalidates dependent analysis before subsequent inference. |
| Presentation | Human and JSON fixtures cover stable formatter, package, LSP, and MCP presentations. |
| Failure preservation | Whole-analysis and language-service regression cases prove no partial signature escapes rejection, retained successful snapshots survive applicable failures, and rejected programs do not run. |

Compiler evidence belongs in the existing semantic test suites and
`examples/specification/check/` with diagnostic companions. Run the existing
executable specification harness and focused package, LSP, and MCP suites for
their respective contracts.

Once the remaining checks pass, update the matching current specifications and
executable examples. Then retire this proposal and its catalog entry.

## Scope And Implementation Basis

This proposal is independently ready. It does not depend on the remaining
refinement-flow, repair, or handler proposals.
It excludes mutable references, declaration-site variance syntax, general
unions/intersections, higher-kinded types, predicate-based subtyping, and
runtime representation changes solely for variance.
