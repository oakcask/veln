# Specification Examples

These examples are executable language fixtures. Each case is ordinary Veln
source plus a small `case.toml` manifest that the CLI toolchain harness runs
against the built `veln` binary.

## Read First

- Use this directory when a behavior should be visible as source code and also
  checked through the public CLI.
- Treat these cases as executable specification evidence for user-visible
  language and CLI behavior. Keep matching prose in `../../docs/specification/`
  aligned with the observable expectations here.
- Prefer adding or improving a case here over expanding prose when the behavior
  is observable through source, diagnostics, command output, or JSON.
- Keep case names grouped by command surface: `check`, `doc`, `fmt`,
  `explain`, `lsp`, `run`, `test`, and `repair`.
- Put the expected observable behavior in `case.toml`; keep the `.veln` files
  readable as examples of the language feature.

## Placement Guidelines

- Add cases here only when the fixture demonstrates a language or public CLI
  behavior that is useful to read as Veln source.
- A runtime case may require the current execution toolchain, but its expected
  behavior must stay phrased as source-level output, diagnostics, exit status,
  or command JSON.
- Do not add cases whose main purpose is to verify backend-private mechanics:
  artifact layout, classfile emission or validation, generated helper names,
  cache reuse, backend-specific limits, host tool setup, or other implementation
  details.
- Put backend invariants in the backend crate tests, and put low-level CLI edge
  cases in the CLI toolchain cases.

## Case Kinds

- `check/`: static diagnostics and successful static validation.
- `doc/`: generated Markdown documentation from source and manifest metadata.
- `fmt/`: deterministic source formatting and whole-invocation write gates.
- `explain/`: diagnostic catalog lookup and command-line errors.
- `lsp/`: editor JSON-RPC behavior exposed by the CLI.
- `run/`: executable entry points, runtime behavior, and runtime failures.
- `test/`: discovered tests, doctests, captured stdio, and test JSON behavior.
- `repair/`: advisory repair preview and repair JSON behavior.
- `package/`: package-manager command workflows and lockfile writes.

## Coverage Map

- `check/source-surface/`: modules, records, dictionaries, vecs, matches,
  qualified `Option` and `Result` constructors, pipelines, wildcard lets,
  record let and match patterns, private inference, parenthesized expressions,
  nested match expressions in call and aggregate positions, and trailing
  record type fields.
- `check/recursive-call-shapes/`: direct recursive calls, nested-match
  tail-shaped recursive calls, postcondition-bearing recursive functions,
  non-tail recursive calls, and function-typed callback calls.
- `check/slash-comments-rejected/`: slash-prefixed comment-like text is
  rejected as source instead of being treated as ordinary or documentation
  comments.
- `check/discovery-parse-gate/`: default recursive source discovery, skipped
  build output, per-file parse gates, and semantic diagnostics from other
  parse-clean files.
- `check/source-metadata/`: ADR-lite doc records attached to source without
  changing static behavior.
- `check/json-ok-envelope/`: successful `check --json` envelope fields and
  zero-diagnostic summary behavior.
- `check/module-imports/`: path-derived modules, `use`, unqualified public
  imports, qualified calls, and qualified pipeline targets.
- `check/local-source-imports/`: source path derived module identity,
  `use foo::bar`, bare public imports, and full-path qualified access.
- `check/local-source-import-boundaries/`: `use foo::bar` does not create a
  short `bar` module alias.
- `check/external-package-imports/`: `use path from "package"` imports public
  names from exported modules in a path dependency.
- `check/external-package-import-boundaries/`: external package imports reject
  private declarations, unexported modules, unavailable packages, and
  dependency manifest package name mismatches.
- `check/source-path-module-diagnostics/`: invalid source path segments for
  derived local module identity.
- `check/import-reexport-boundary/`: `use` declarations let a module consume
  another module's public API without publishing that API through the consuming
  module's own qualified path.
- `check/public-member-alias-reexports/`: `pub fn` and `pub type` member
  aliases publish imported implementation members through the declaring
  module's public path.
- `check/public-member-alias-diagnostics/`: public member aliases reject
  wrong-kind targets, unresolved targets, and duplicate exported names.
- `check/qualified-no-fallback/`: qualified calls require a matching import
  alias and do not fall back to same-named bare functions.
- `check/missing-module-identity/`: dotted module delimiters in `use`
  declarations are rejected.
- `check/name-module-boundaries/`: duplicate import aliases, functions,
  parameters, local bindings, record fields, and pattern bindings.
- `check/parse-recovery-diagnostics/`: parse recovery diagnostics for call
  arguments, missing newlines, and malformed let patterns.
- `check/type-delimiter-diagnostics/`: legacy type delimiters report ordinary
  parse or type diagnostics without delimiter replacement candidates.
- `check/predicate-pattern-diagnostics/`: unsupported contract and `satisfy`
  predicate syntax, malformed `satisfy` suffixes, refutable `let` patterns,
  and invalid `satisfy` candidate bindings.
- `check/contract-validation-diagnostics/`: contract predicates that parse but
  fail static validation for effectful calls, non-boolean facts, and missing
  fields.
- `check/type-effect-boundaries/`: private inference gaps, missing record
  fields, `Path` versus `String`, invalid pipeline targets,
  method-call-shaped syntax, unknown effects, and indirect effect inference.
- `check/function-effect-boundaries/`: function-typed value compatibility when
  callable effects are narrower or wider than the expected function type.
- `check/type-annotation-boundaries/`: public API annotation requirements,
  invalid type annotations, and top-level test declaration shape requirements.
- `check/named-type-annotations/`: non-built-in named type paths with type
  arguments inside value and function type annotations.
- `check/source-adt-boundaries/`: source-declared ADT constructor namespace
  conflicts, same-module type-qualified disambiguation, import visibility,
  local private constructor paths, hidden-constructor exhaustiveness, nullary
  generic constructor context, and current `List`/`Vec` conversion helper
  boundaries.
- `check/source-adt-exhaustiveness/`: source-declared ADT finite-domain
  matching reports unqualified missing constructor coverage labels.
- `check/manifest-exports/`: `[lib].exports` accepts selected source-file
  paths and rejects module-path spelling, paths outside the package, missing
  files, non-source files, invalid path-derived modules, duplicate module
  exports, and unselected source files.
- `check/manifest-dependencies/`: git dependency metadata requires exactly
  one selector among `rev`, `tag`, and `branch`.
- `package/lock-path-dependencies/`: `veln package lock` writes a sorted
  `veln.lock` for available path dependencies, records identity separately
  from path source, computes `sha256:` source-tree checksums, and ignores
  build output.
- `package/lock-vendor-dependency/`: `veln package lock` writes a vendor
  source record for an already available vendored package directory, validates
  the dependency manifest package identity, and computes the source-tree
  checksum.
- `package/lock-vendor-package-name-mismatch/`: `veln package lock` rejects a
  vendor dependency whose manifest `[package].name` does not match the
  dependency table key.
- `package/lock-mirror-dependency/`: `veln package lock` writes a mirror
  source record for an already materialized mirror source tree while preserving
  the dependency table key as the package identity.
- `package/lock-mirror-unavailable/`: `veln package lock` requires explicit
  mirror metadata to name an already materialized package source tree.
- `package/lock-git-rev-dependency/`: `veln package lock` writes a `rev`
  git source record for an already available local repository, validates a
  dependency `subdir` package root, records the resolved commit, and checksums
  only the selected package source tree.
- `package/lock-git-remote-rev-dependency/`: `veln package lock` materializes
  a non-local git URL before writing a `rev` git source record that preserves
  the original URL, resolved commit, `subdir`, and selected source-tree
  checksum.
- `package/lock-git-remote-tag-dependency/`: `veln package lock`
  materializes a non-local git URL before writing a `tag` git source record.
- `package/lock-git-remote-branch-dependency/`: `veln package lock`
  materializes a non-local git URL before writing a `branch` git source record.
- `package/lock-git-tag-dependency/`: `veln package lock` preserves a
  requested `tag` selector while recording the resolved commit separately.
- `package/lock-git-branch-dependency/`: `veln package lock` preserves a
  requested `branch` selector while recording the resolved commit separately.
- `package/lock-package-name-mismatch/`: `veln package lock` rejects a path
  dependency whose manifest `[package].name` does not match the dependency
  table key.
- `package/lock-mirror-package-name-mismatch/`: `veln package lock` rejects a
  mirror dependency whose manifest `[package].name` does not match the
  dependency table key and reports the dependency manifest name as related
  context.
- `package/lock-incompatible-transitive-path-source/`: `veln package lock`
  rejects a dependency graph that selects different path sources for one
  package identity.
- `package/lock-incompatible-transitive-source-kind/`: `veln package lock`
  rejects a dependency graph that selects the same source location through
  incompatible source kinds for one package identity.
- `package/lock-incompatible-transitive-git-selector/`: `veln package lock`
  rejects a dependency graph that selects incompatible git selectors for one
  package identity.
- `package/lock-incompatible-transitive-git-subdir/`: `veln package lock`
  rejects a dependency graph that selects incompatible git subdirectories for
  one package identity.
- `check/implicit-unit-return/`: omitted tail expressions returning `()` and
  the implicit-unit diagnostic detail.
- `check/types-operators/`: primitive annotations, returned function types,
  boolean and float operators, Bool matches, and qualified Result constructors.
- `check/checked-core-blockers/`: checked-core executable blockers for missing
  expressions plus call and constructor arity mismatches.
- `check/contracts-result-binding/`: `require`, `ensure`, `invariant`,
  explicit result bindings, and pure prelude calls in predicates.
- `check/contract-result-binding-scope/`: explicit result bindings are visible
  only to same-function postconditions, and bare `result` is ordinary.
- `check/contract-predicate-calls/`: contract predicates with alias-qualified
  pure calls, pure call return fields, numeric pure-call results, and function
  declaration values passed to predicate helpers.
- `check/contract-static-classification/`: statically proven literal,
  tautological, and same-shape contract predicates.
- `check/match-non-exhaustive/`: finite-domain match exhaustiveness
  diagnostics.
- `check/match-result-non-exhaustive/`: Result finite-domain match
  exhaustiveness diagnostics.
- `check/effect-missing-public/`: public effect-boundary diagnostics.
- `check/effect-empty-declaration/`: declaration-level `effects []`
  diagnostics and pure test omission.
- `check/effect-reserved-labels/`: reserved public effect labels accepted as
  declared compatibility boundaries.
- `check/hole-satisfy/`: typed holes with `satisfy` repair constraints.
- `check/named-hole-labels/`: named hole labels remain repair metadata and do
  not become value declarations.
- `check/human-ok/`: human `check` output for valid input.
- `check/prelude-helper-diagnostics/`: fallible `vec_map` callback diagnostics
  and repair hints toward `vec_try_map`.
- `check/implicit-prelude-imports/`: implicit standard prelude import
  ambiguity with written imports and reserved `prelude` aliases and module
  identities.
- `check/implicit-prelude-qualified-imports/`: qualified written-import and
  `prelude::` selection when both export the same bare helper name.
- `check/doctest-static-examples/`: documentation-only doctest fences and
  negative static doctest fences.
- `check/doctest-metadata-diagnostics/`: unknown and invalid doctest metadata,
  duplicate output fences, and missing expected failures.
- `check/schema-declarations/`: accepted top-level `schema` and `pub schema`
  declarations with `format binary` fields, exact-width unsigned primitive
  fields, `ReservedBits(width, value)` primitive fields, and field-local
  `where` predicates.
- `check/schema-declaration-diagnostics/`: parser diagnostics for malformed
  schema headers, missing `end`, fields before `format`, multiple `format`
  clauses, `_`-prefixed fields, and malformed field-local `where`
  predicates.
- `check/schema-reserved-bits-diagnostics/`: declaration diagnostics for
  malformed `ReservedBits(width, value)` primitive arguments.
- `check/schema-exact-width-primitive-diagnostics/`: declaration diagnostics
  when exact-width unsigned primitive names are used as ordinary source types
  or values.
- `check/schema-ordinary-use-diagnostics/`: schema declarations do not create
  ordinary value bindings.
- `check/codec-declarations/`: accepted private and public top-level `codec`
  declarations with explicit `decode` and `encode` directions, `derive`
  clauses, and `with` clauses.
- `check/codec-declaration-diagnostics/`: parser diagnostics for empty,
  duplicate, and unknown codec direction lists plus missing, unlisted, and
  duplicate implementation clauses.
- `check/codec-decode-signature-diagnostics/`: `decode with` checker
  diagnostics for unresolved decoder functions and wrong decode function
  parameter or return shapes.
- `check/codec-decode-signature-human/`: human `check` output keeps the
  `codec.decode_signature` primary message at the codec implementation clause
  and reports the referenced function as a related note.
- `doc/generated-markdown/`: generated documentation from package and tool
  metadata, module identity, imports, public functions, contracts, doctest
  fences, hidden doctest setup, and ADR-lite records.
- `doc/manifest-modules-rejected/`: generated documentation is blocked when a
  manifest uses the rejected `[modules]` table.
- `doc/no-selected-sources/`: generated documentation can contain only
  package metadata and an empty module section when no source files are
  selected.
- `doc/parse-gate/`: generated documentation is blocked by selected-source
  parse diagnostics before writing Markdown.
- `fmt/canonical-formatting/`: headers, imports, standalone and trailing
  comments, contracts, match indentation, operators, postfix `?`, lists,
  records, and idempotence.
- `fmt/schema-declarations/`: canonical layout for schema headers,
  `format binary`, fields, field-local `where` predicates, and idempotence.
- `fmt/codec-declarations/`: canonical layout for codec headers, direction
  lists, implementation clauses, and idempotence.
- `fmt/all-or-nothing/`: parse-failure write gate across multiple files.
- `explain/known-diagnostic/`: known diagnostic explanation output.
- `explain/list-catalog/`: implemented diagnostic catalog listing.
- `explain/unknown-diagnostic/`: unknown diagnostic command-line error.
- `explain/missing-diagnostic/`: missing diagnostic command-line error.
- `lsp/semantic-tokens/`: JSON-RPC initialize, didOpen, semantic tokens,
  shutdown, and exit over stdin.
- `lsp/semantic-tokens-unsaved-change/`: semantic tokens follow unsaved
  document content supplied by didChange.
- `lsp/schema-semantic-tokens/`: semantic-token transport for schema
  declarations and format clauses.
- `lsp/publish-diagnostics/`: diagnostics are published from open document
  text and cleared after didClose.
- `lsp/unopened-missing-file/`: semantic-token requests for unopened,
  unreadable documents return an empty token data array.
- `lsp/workspace-diagnostics/`: LSP initialize with `rootUri` publishes
  workspace diagnostics for discovered unopened files using cross-file checker
  behavior.
- `run/stdio-streams/`: `stdio::print`, `stdio::println`, `stdio::eprint`,
  and `stdio::eprintln` stream behavior.
- `run/prelude-helpers/`: result-bearing prelude traversal helpers and
  runtime stdio.
- `run/prelude-containers/`: vec, dictionary, list, option, result, and string
  prelude helper value semantics, including non-mutating container updates,
  source-order vec and list traversal, and empty-container checks.
- `run/byte-fixture-hex/`: compact ASCII hex fixture text, whitespace between
  byte pairs, and decoded `ByteChunk` values.
- `run/byte-fixture-hex-invalid/`: invalid compact hex fixture characters,
  prefixes, or separators fail with stable fixture hex error text.
- `run/byte-fixture-hex-invalid-human/`: invalid compact hex fixture text
  propagates stable fixture hex error text through human `run` stderr.
- `run/byte-fixture-hex-invalid-json/`: invalid compact hex fixture text
  propagates stable fixture hex error text through `run --json` stderr and
  structured fixture text validation details through JSON.
- `run/byte-fixture-hex-odd/`: dangling compact hex fixture nibbles fail with
  stable fixture hex error text.
- `run/byte-fixture-hex-odd-human/`: dangling compact hex fixture nibbles
  propagate stable fixture hex error text through human `run` stderr.
- `run/byte-fixture-hex-odd-json/`: dangling compact hex fixture nibbles
  propagate stable fixture hex error text through `run --json` stderr and
  structured fixture text validation details through JSON.
- `run/binary-fixture-records/`: test-owned named binary fixture records carry
  decoded byte chunks built in fixture source, optional consumed counts, and
  invalid fixture expectations; `case.toml` checks exact complete lowercase
  hex output plus named output chunk lists, including multi-chunk order,
  zero-length chunks, and empty lists.
- `run/binary-fixture-truncated-input-json/`: a named binary fixture record
  decodes valid compact hex bytes that are intentionally too short for a
  fixed-width `ByteView` read; `case.toml` checks fixture-owned truncation
  facts separately from the `codec.incomplete_input` JSON details.
- `run/binary-fixture-invalid-field/`: a named binary fixture record decodes
  valid compact hex bytes and records a test-owned invalid field check with a
  diagnostic id, byte offset, structured field path, and consumed count.
- `run/binary-byteview/`: `ByteView` slices, fixed-width unsigned big-endian
  reads and writes, truncation failures, range failures, and conversion
  overflow failures, plus channel freeze preservation for bounded views.
- `run/binary-byteview-read-failure-json/`: ByteView read truncation propagates
  as a runtime `Result` failure through `run --json` with
  `codec.incomplete_input` byte diagnostic details, including byte offset,
  field path, byte counts, and readiness.
- `run/binary-byteview-read-failure-human/`: ByteView read truncation
  propagates through human `run` stderr as a `codec.incomplete_input`
  diagnostic with the missing byte offset and related readiness and byte count
  context.
- `run/binary-byteview-range-failure-human/`: ByteView range failures propagate
  stable error text through human `run` stderr.
- `run/binary-byteview-write-failure-human/`: unsigned big-endian write
  overflow propagates stable error text through human `run` stderr.
- `run/binary-schema-frame-header-decode/`: the implemented binary schema
  primitive decode slice consumes a frame-header `ByteView`, returns visible
  exact-width fields as ordinary `Int` values, and omits the reserved field
  from the mapped record.
- `run/binary-schema-frame-header-truncated-json/`: frame-header schema decode
  truncation reports `schema.truncated_field` through `run --json` with byte
  offset, field path, byte counts, readiness, and nearby bytes.
- `run/binary-schema-frame-header-reserved-json/`: frame-header reserved-bit
  validation reports `schema.reserved_bits_mismatch` through `run --json`
  with byte offset, field path, bit width, expected value, actual value, and
  nearby bytes.
- `run/codec-decode-step-vocabulary/`: ordinary source constructs and matches
  `DecodeStep<T>`, `DecodeReadiness`, and `DecodeError` values for decoded,
  need-more-input, and invalid-input decoder outcomes.
- `run/http2-protocol-core/`: an ordinary-source HTTP/2 sans-I/O decode state
  handles chunk arrival, incomplete input, end-of-stream truncation, and a
  continuation ordering failure while projecting typed protocol failures into
  stable ids and related context.
- `run/http2-protocol-core-closed-human/`: closed HTTP/2 input with undecoded
  pending bytes reports `http2.protocol.closed_with_pending` through human
  `run` stderr with byte offset, pending byte count, and active continuation
  context.
- `run/http2-protocol-core-continuation-json/`: a continuation ordering
  failure reports `http2.protocol.continuation_expected` through `run --json`
  with byte offset, frame kind, stream id, and active continuation details.
- `run/stream-input-vocabulary/`: `StreamInput` construction and matching for
  chunk arrivals, empty chunks, explicit end events, and qualified prelude
  constructor paths.
- `run/implicit-prelude-imports/`: qualified `prelude::` fallback and local
  declaration shadowing over implicit prelude imports.
- `run/tail-recursion-trampoline/`: stack-safe direct tail recursion, nested
  match tail positions, runtime `require` checks, and argument evaluation
  before parameter rebinding.
- `run/result-propagation/`: `Result` propagation, dictionary lookup, function
  values, and runtime JSON success.
- `run/match-source-order/`: match arms are evaluated in source order.
- `run/source-adts/`: source-declared ADT construction, matching, runtime
  output, and record-shaped variant payload order.
- `run/selected-reachability/`: selected-entry reachability, ignored
  unreachable semantic errors, imported function values, function-typed local
  calls, and local shadowing of same-named function declarations.
- `run/entry-arguments/`: selected run entry conversion for `String`, `Int`,
  `Float`, and `Bool` command-line arguments.
- `run/missing-entry-gate/`: missing run entry names fail before execution.
- `run/entry-argument-count-gate/`: run entry argument arity is checked before
  execution.
- `run/unsupported-entry-argument-type/`: unsupported run entry parameter types
  are rejected before conversion.
- `run/invalid-entry-argument/`: invalid command-line text for a supported
  entry parameter type reports the named conversion error.
- `run/contract-ensure-failure/`: runtime `ensure` failure details and
  implementation blame.
- `run/contract-ensure-early-return/`: `ensure` checks before `?` early
  returns.
- `run/contract-invariant-failure/`: runtime `invariant` failure details and
  caller blame at entry.
- `run/contract-reachability-blockers/`: selected run reachability through
  pure helpers and function values used only by contract predicates.
- `run/standard-effects/`: process and file-system standard calls with
  declared effect boundaries, including present and missing environment
  lookups.
- `run/concurrency-boundary/`: task and channel standard calls with declared
  concurrency effects, including explicit and inferred item types, sender
  closing, closed receives, and failed sends.
- `run/concurrency-selection/`: receive, priority selection, timeout selection,
  non-priority selection rotation, and task cancellation under the concurrency
  effect.
- `run/concurrency-result-selection/`: fallible channel result selection and
  priority-result selection.
- `run/file-system-values/`: file-system result behavior for current-directory
  existence, directory reads, and file-operation error values.
- `run/process-exit-status/`: `process::exit` status propagation.
- `run/process-exit-json/`: `run --json` records non-zero process exits as
  runtime errors.
- `run/human-stdio/`: human `run` mode forwards stdout and stderr from the
  selected program.
- `run/line-item-order-summary/`: the implemented line-item order summary
  example using dictionary lookup, fallible traversal, folding, records,
  `Result` propagation, and stdio output.
- `test/discovered-tests/`: targetless discovery of conventional test files
  and non-test files containing top-level tests.
- `test/no-discovered-json/`: targetless test discovery reports a blocked JSON
  suite error when no top-level tests are discovered.
- `test/top-level-tests/`: top-level `test` declarations, including
  `Result<(), E>` tests and captured stdio event JSON, selected through the
  public test command.
- `test/doctest-output/`: doctest expected-output fences.
- `test/doctest-result-metadata/`: doctest `error` metadata and hidden setup.
- `test/doctest-result-inference/`: doctest `Result` wrapper inference from
  documented APIs or propagated calls, plus stderr expected-output fences.
- `test/static-gate-blocked-json/`: static gates block discovered cases in
  JSON output.
- `test/doctest-output-mismatch-json/`: expected-output mismatch failure
  details, including first-difference and captured-event records.
- `test/doctest-runtime-contract-json/`: runtime contract expectation matching
  for positive doctests, including matching failures, mismatched failure
  details, missing failures, and adjacent expected output.
- `test/doctest-runtime-contract-blocked-json/`: the matching static-gate case
  that blocks a doctest runtime contract expectation before execution.
- `test/runtime-contract-failure-json/`: runtime contract failure details
  inside a selected test case.
- `test/source-to-test-convention/`: explicit source targets selecting a paired
  same-directory test file.
- `repair/hole-preview/`: `repair --json` preview records for advisory hole
  candidates.
- `repair/apply-safe-candidate/`: `repair --apply` writes one safe candidate
  and verifies the result.
- `repair/apply-confirmed-override/`: confirmed override applies and records a
  manual-review candidate.
- `repair/verification-checked-core-rollback/`: repair verification rolls back
  when shared check analysis reports a checked-core blocker.
- `repair/refuse-multiple-candidates/`: automatic apply refuses ambiguous safe
  candidates until one is selected.
- `repair/refuse-override-without-confirm/`: override application requires
  explicit confirmation.
- `repair/saved-preview-normalization/`: saved repair JSON input is normalized
  to command-level preview candidates.
- `repair/saved-apply-requires-current-match/`: saved repair JSON does not
  authorize writes without a current safe candidate match.

## Boundaries

These cases complement `crates/veln-cli/tests/toolchain_cases/`. Use this
directory when source readability is part of the value of the test. Keep
low-level CLI edge cases in the CLI test directory.
