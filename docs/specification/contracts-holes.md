# Contracts And Holes

This page is the routing entry for implemented contract and hole behavior.
Read the focused specification pages first; open the full detail files only when
you need exact rules or examples.

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
- Use [contracts-full.md](contracts-full.md) or [holes-full.md](holes-full.md)
  when a task needs exact rules.
- Use [contracts-holes-full.md](contracts-holes-full.md) only when a broad
  search starts from the former combined detail path.

## Skip Unless Needed

- Do not read a full detail file when a focused contract or hole route page
  answers the behavior question.
- Do not treat proposal text as implemented behavior unless these reference
  pages also state it.
