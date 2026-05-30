# Agent Test Selection Graph

Status: proposed

This page tracks follow-up work for dependency-aware test selection. Proposal
text here is not current test behavior unless `../specification/` also states
it.

## Read First

- Current command behavior:
  [../specification/commands.md](../specification/commands.md).
- Current test JSON:
  [../specification/test-json.md](../specification/test-json.md).
- Current source and module behavior:
  [../specification/source-surface.md](../specification/source-surface.md).

## Current Boundary

`veln test` discovers explicit `test` declarations and executable doctests. An
explicit non-test source target may add a same-directory paired test file by
convention. That route reports partial confidence because it is conservative
and narrower than a complete dependency graph.

## Proposed Target

Add dependency-aware test selection that prefers false positives over false
negatives. When the graph is incomplete, the tool should report the missing
evidence and widen selection instead of silently under-selecting tests.

## Open Work

- Define the source-level dependency graph inputs that are stable enough for
  command behavior.
- Decide whether graph inspection is part of `test`, a future `graph` command,
  or both.
- Extend JSON selection metadata so tools can distinguish exact, widened, and
  unknown selections.
- Keep doctest and explicit target behavior compatible with the broader graph.

## Non-Targets

- Do not replace current discovery or paired-test behavior until graph-backed
  selection has implementation and test coverage.
- Do not claim complete selection confidence when imports, generated doctests,
  dynamic reachability, or package metadata make the graph incomplete.
