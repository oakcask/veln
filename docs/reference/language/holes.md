# Holes

This page routes hole-specific language behavior. It points into the full
combined detail until the hole body is split further.

## Read First

- [Hole diagnostics and details](contracts-holes-full.md#hole-diagnostics) defines
  `hole.unfilled`, partial check status, and stable diagnostic detail fields.
- [Repair candidates](contracts-holes-full.md#repair-candidates) defines
  candidate query records, ranking, application policy, and safe repair
  candidates.
- [Satisfy constraints](contracts-holes-full.md#satisfy-constraints) defines `satisfy`
  parsing, candidate scoping, validation, and static satisfaction.

## Read When

- Use this page before changing hole parsing, expected-type flow, diagnostic
  details, candidate ranking, safe repair policy, or satisfy validation.
- Use [contracts.md](contracts.md) only when a hole change reuses contract
  predicate validation or `require` evidence.

## Skip Unless Needed

- Skip the former combined detail unless you need exact rules or examples.
- Use [diagnostics-json.md](diagnostics-json.md#diagnostics) for stable JSON
  diagnostics before reading older decision records.
