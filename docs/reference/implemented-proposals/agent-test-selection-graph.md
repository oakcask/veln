# Agent Test Selection Graph

Status: implemented

This page records completion evidence for dependency-aware test selection. Use
the specification pages for current command behavior.

## Read First

- Current command behavior:
  [../../specification/commands.md](../../specification/commands.md).
- Current test JSON:
  [../../specification/test-json.md](../../specification/test-json.md).
- Planned work now only belongs in
  [../../proposals/README.md](../../proposals/README.md).

## Outcome

`veln test` keeps direct explicit test selection, same-directory
source-to-test pairing, and doctest selection compatible with the previous
behavior. For explicit non-test source targets, it now builds a source-level
graph from `mod` and `use` declarations and adds discovered tests whose
transitive imports include the selected source.

When graph evidence is incomplete, the command reports the missing evidence in
selection notes, marks selection confidence as `unknown`, and widens to all
discovered tests so the selection prefers false positives over false
negatives.

## Completion Evidence

- JSON selection metadata distinguishes exact graph selection from widened
  graph selection through `reason` and `confidence`.
- Source-to-test convention notes remain visible in human and JSON output.
- Explicit source targets with module identities can select importing test
  files without naming those test files directly.
- Explicit source targets without module identities widen to all discovered
  tests and report the missing module evidence.
- Doctest and explicit target selection use selected roots separately from the
  analysis dependency closure.

## Read When

- Checking why dependency-aware test selection is no longer listed as active
  proposal work.
- Reviewing completion evidence before changing graph selection behavior.
