# Specification Examples

These examples are executable language fixtures. Each case is ordinary Veln
source plus a small `case.toml` manifest that the CLI toolchain harness runs
against the built `veln` binary.

## Read First

- Use this directory when a behavior should be visible as source code and also
  checked through the public CLI.
- Keep case names grouped by command surface: `check`, `fmt`, `explain`,
  `lsp`, `run`, `test`, and `repair`.
- Put the expected observable behavior in `case.toml`; keep the `.veln` files
  readable as examples of the language feature.

## Case Kinds

- `check/`: static diagnostics and successful static validation.
- `fmt/`: deterministic source formatting and whole-invocation write gates.
- `explain/`: diagnostic catalog lookup and command-line errors.
- `lsp/`: editor JSON-RPC behavior exposed by the CLI.
- `run/`: executable entry points, runtime behavior, and runtime failures.
- `test/`: discovered tests, doctests, captured stdio, and test JSON behavior.
- `repair/`: advisory repair preview and repair JSON behavior.

## Coverage Map

- `check/source-surface/`: modules, records, dictionaries, vecs, matches,
  pipelines, wildcard lets, and record let patterns.
- `check/module-imports/`: `mod`, `use`, import aliases, and qualified calls.
- `check/missing-module-identity/`: `use` declarations without a source module
  identity.
- `check/name-module-boundaries/`: duplicate import aliases, functions,
  parameters, local bindings, record fields, and pattern bindings.
- `check/parse-recovery-diagnostics/`: parse recovery diagnostics for call
  arguments, missing newlines, and malformed let patterns.
- `check/type-effect-boundaries/`: private inference gaps, missing record
  fields, `Path` versus `String`, invalid pipeline targets,
  method-call-shaped syntax, unknown effects, and indirect effect inference.
- `check/manifest-metadata/`: source `mod` ownership wins over manifest module
  metadata.
- `check/types-operators/`: primitive annotations, returned function types,
  boolean and float operators, Bool matches, and qualified Result constructors.
- `check/contracts-result-binding/`: `require`, `ensure`, `invariant`,
  explicit result bindings, and pure prelude calls in predicates.
- `check/contract-static-classification/`: statically proven literal,
  tautological, and same-shape contract predicates.
- `check/match-non-exhaustive/`: finite-domain match exhaustiveness
  diagnostics.
- `check/match-result-non-exhaustive/`: Result finite-domain match
  exhaustiveness diagnostics.
- `check/effect-missing-public/`: public effect-boundary diagnostics.
- `check/hole-satisfy/`: typed holes with `satisfy` repair constraints.
- `check/doctest-static-examples/`: documentation-only doctest fences and
  negative static doctest fences.
- `check/doctest-metadata-diagnostics/`: unknown and invalid doctest metadata,
  duplicate output fences, and missing expected failures.
- `fmt/canonical-formatting/`: headers, imports, comments, contracts, match
  indentation, operators, postfix `?`, lists, records, and idempotence.
- `fmt/all-or-nothing/`: parse-failure write gate across multiple files.
- `explain/known-diagnostic/`: known diagnostic explanation output.
- `explain/list-catalog/`: implemented diagnostic catalog listing.
- `explain/unknown-diagnostic/`: unknown diagnostic command-line error.
- `explain/missing-diagnostic/`: missing diagnostic command-line error.
- `lsp/semantic-tokens/`: JSON-RPC initialize, didOpen, semantic tokens,
  shutdown, and exit over stdin.
- `run/stdio-streams/`: `stdio::print`, `stdio::println`, `stdio::eprint`,
  and `stdio::eprintln` stream behavior.
- `run/prelude-helpers/`: result-bearing prelude traversal helpers and
  runtime stdio.
- `run/prelude-containers/`: vec, dictionary, option, result, and string
  prelude helper value semantics.
- `run/result-propagation/`: `Result` propagation, dictionary lookup, function
  values, and runtime JSON success.
- `run/entry-arguments/`: selected run entry conversion for `String`, `Int`,
  `Float`, and `Bool` command-line arguments.
- `run/contract-ensure-failure/`: runtime `ensure` failure details and
  implementation blame.
- `run/contract-invariant-failure/`: runtime `invariant` failure details.
- `run/standard-effects/`: process and file-system standard calls with
  declared effect boundaries.
- `run/concurrency-boundary/`: task and channel standard calls with
  declared concurrency effects.
- `run/concurrency-selection/`: receive, priority selection, timeout selection,
  and task cancellation under the concurrency effect.
- `run/concurrency-result-selection/`: fallible channel result selection and
  priority-result selection.
- `run/file-system-values/`: file-system result behavior for current-directory
  existence, directory reads, and file-operation error values.
- `run/process-exit-status/`: `process::exit` status propagation.
- `run/cache-stability/`: repeated JVM-backed execution through the public
  cache boundary.
- `test/top-level-tests/`: top-level `test` declarations selected through the
  public test command.
- `test/doctest-output/`: doctest expected-output fences.
- `test/doctest-result-metadata/`: doctest `error` metadata and hidden setup.
- `test/static-gate-blocked-json/`: static gates block discovered cases in
  JSON output.
- `test/doctest-output-mismatch-json/`: expected-output mismatch failure
  details.
- `test/runtime-contract-failure-json/`: runtime contract failure details
  inside a selected test case.
- `test/source-to-test-convention/`: explicit source targets selecting a paired
  same-directory test file.
- `repair/hole-preview/`: `repair --json` preview records for advisory hole
  candidates.
- `repair/apply-safe-candidate/`: `repair --apply` writes one safe candidate
  and verifies the result.
- `repair/refuse-multiple-candidates/`: automatic apply refuses ambiguous safe
  candidates until one is selected.
- `repair/refuse-override-without-confirm/`: override application requires
  explicit confirmation.
- `repair/saved-preview-normalization/`: saved repair JSON input is normalized
  to command-level preview candidates.

## Boundaries

These cases complement `crates/veln-cli/tests/toolchain_cases/`. Use this
directory when source readability is part of the value of the test. Keep
low-level CLI edge cases in the CLI test directory.
