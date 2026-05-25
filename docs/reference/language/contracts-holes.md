# Contracts And Holes

This page is the routing entry for implemented contract and hole behavior.
Read the focused reference pages first; open the full combined detail only
when you need the historical specification body in its original order.

## Read First

- [contracts.md](contracts.md): implemented contract clauses, predicate
  validation, runtime obligation classification, blame, and explicit result
  bindings.
- [holes.md](holes.md): implemented hole diagnostics, repair candidate records,
  `satisfy` constraints, safe repair policy, and satisfy predicate validation.

## Read When

- Use [contracts.md](contracts.md) before changing contract parsing, checking,
  runtime enforcement, or diagnostics.
- Use [holes.md](holes.md) before changing hole diagnostics, expected-type flow,
  candidate ranking, or safe repair behavior.
- Use [contracts-holes-full.md](contracts-holes-full.md) when a task needs the
  former combined specification text or a broad search across both topics.

## Skip Unless Needed

- Do not read the full combined detail when a focused contract or hole page
  answers the behavior question.
- Do not treat proposal text as implemented behavior unless these reference
  pages also state it.
