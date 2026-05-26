# Holes

This page routes hole-specific language behavior. Open the full detail only for
exact rules or examples.

## Read First

- [Hole diagnostics and details](holes-full.md#hole-diagnostics) defines
  `hole.unfilled`, partial check status, and stable diagnostic detail fields.
- [Repair candidates](holes-full.md#repair-candidates) defines
  candidate query records, ranking, application policy, and safe repair
  candidates, including boolean alias, predicate implication, and adjacent
  integer-bound cases. Statically satisfied `satisfy` repair candidates remain
  visible even when ordinary manual-review candidates are bounded.
- [Satisfy constraints](holes-full.md#satisfy-constraints) defines `satisfy`
  parsing, candidate scoping, validation, and static satisfaction.

## Read When

- Use this page before changing hole parsing, expected-type flow, diagnostic
  details, candidate ranking, safe repair policy, or satisfy validation.
- Use [contracts.md](contracts.md) only when a hole change reuses contract
  predicate validation or `require` evidence.

## Skip Unless Needed

- Skip [holes-full.md](holes-full.md) unless you need exact rules or examples.
- Use [diagnostics-json.md](diagnostics-json.md#diagnostics) for stable JSON
  diagnostics before reading older decision records.
