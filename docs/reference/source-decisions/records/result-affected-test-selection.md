# Discussion Result: Affected Test Selection

Status: implemented

## Picked Question

- How conservative should affected-test selection be before a full dependency
  graph exists?

## Decision

Before a full dependency graph exists, affected-test selection should be
conservative in favor of running too many tests rather than too few.

`veln test` may run a narrowed set only when the changed files map to tests
through explicit, trusted evidence: a direct file list from the user, a test
target named on the command line, a parsed same-file example, or an unambiguous
first-slice source-to-test convention. When the mapping is incomplete,
ambiguous, or unavailable, the command should widen to the next trustworthy
scope and report why it widened.

## Rationale

The first implementation is meant to shorten repair loops, but an apparently
fast test result is harmful if it hides a relevant failure. Agents are likely to
treat a focused green run as permission to continue, so the early tool should
make uncertainty explicit instead of presenting a weak dependency guess as a
precise affected-test set.

This follows the existing first-slice direction: `graph` is not a required
first command, `check` may report `status: partial`, and hole reachability
already treats unavailable graph information as possibly reachable. Test
selection should use the same trust model. Lack of graph precision is allowed;
silent under-selection is not.

The command can still protect small edit loops. Explicit test selection,
same-file examples, and simple source/test naming conventions are enough for
useful first-slice behavior. The important boundary is that every narrowed run
must say what evidence selected the tests and whether the result should be
treated as complete for the edited scope.

## First-Slice Rules

- `veln test` may accept explicit test targets and treat them as intentionally
  narrowed by the caller.
- Automatic affected-test selection must include a confidence state in human
  and JSON output, at least `complete`, `partial`, or `unknown`.
- If dependency information is `partial` or `unknown`, automatic selection
  widens to the smallest broader scope the tool can justify, such as the whole
  package, all explicit test files, or all discovered examples.
- The widened run should include a note explaining which dependency evidence
  was missing or ambiguous.
- A narrowed automatic run must not be reported as a complete verification when
  imports, module metadata, generated docs, contracts, or examples that could
  affect the changed code were not analyzed.
- CI-oriented defaults should prefer full tests until the dependency graph is
  precise enough to make affected selection auditable.

## Open Detail

The exact first set of source-to-test conventions can stay small. A direct
`*_test.veln` pairing, same-file examples, and explicit command-line targets
are compatible with this decision. Package manifests and a later `graph`
command can refine the evidence model without changing the conservative
fallback rule.

The test-result JSON shape is resolved by
[Test JSON Shape](result-test-json-shape.md). Selection policy still remains
independent of the exact reporting schema, but the JSON result now carries
selection confidence and widening reasons.

## Consequence

First-slice test selection remains useful without creating false confidence.
Agents get fast focused runs when the evidence is explicit, and broader runs
when the tool cannot prove the narrowed set is complete.
