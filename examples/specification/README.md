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
  fields,
  `ReservedBits(width, value)` primitive fields, and field-local `where`
  predicates, plus structural `map to Target` clauses that assign schema-local
  fields to target fields.
- `check/binary-schema-u16le/`: `UInt16le`, `UInt24le`, `UInt31le`, and
  `UInt32le` are accepted as `format binary` schema field primitives on
  private and public schema declarations.
- `check/schema-declaration-diagnostics/`: parser diagnostics for malformed
  schema headers, missing `end`, fields before `format`, multiple `format`
  clauses, `_`-prefixed fields, malformed field-local `where` predicates,
  and malformed schema mapping clauses.
- `check/schema-declaration-human/`: malformed field-local schema `where`
  syntax reports a human `check` parse diagnostic instead of a runtime schema
  validation diagnostic.
- `check/schema-mapping-diagnostics/`: generated binary schema mapping
  diagnostics for mapping assignments that name schema-local source fields
  not decoded by the schema.
- `check/schema-packed-reserved-mapping-diagnostics/`: packed
  `ReservedBits(width, value)` fields are representation-only and are not
  available as structural mapping source fields.
- `check/schema-mapping-selection-diagnostics/`: binary schema diagnostics for
  missing, duplicate, overlapping, and unsupported structural mapping
  selection.
- `check/schema-mapping-arithmetic-literals/`: schema mapping arithmetic
  accepts integer literal operands for `Int` target fields.
- `run/binary-schema-mapping-selection-decode/`: generated binary schema
  decode selects one structural mapping from an already decoded field value.
- `run/binary-schema-mapping-selection-not-equal-decode/`: generated binary
  schema decode selects a structural mapping with `when field != literal`.
- `run/binary-schema-mapped-field-selection-decode/`: generated binary schema
  decode maps a selected field from an already decoded nested record value.
- `run/binary-schema-mixed-dispatch-selected-mapping-decode/`: generated
  binary schema decode accepts a closed dispatch whose primitive and nested
  payload cases are wrapped by selected mappings into one target record shape.
- `run/binary-schema-mapping-arithmetic-decode/`: generated binary schema
  decode computes `Int` target fields with supported decoded-field and
  integer-literal `+`, `-`, and `*` mapping arithmetic.
- `check/schema-mapping-field-selection-diagnostics/`: field-selection schema
  mapping expressions reject missing selected fields and non-record selection
  targets.
- `check/schema-mapping-expression-boundary-diagnostics/`: record and
  constructor-shaped schema mapping assignment values report unsupported
  expression, unresolved constructor, constructor arity, payload type,
  non-`Int` arithmetic operand, and unsupported arithmetic operand diagnostics
  when they exceed the implemented structural expression slice.
- `check/schema-mapping-converter-diagnostics/`: converter-shaped schema
  mapping assignment values report unresolved converter, arity, input type,
  return type, purity, and unsupported converter argument expression
  diagnostics.
- `check/schema-imported-mapping-converter-diagnostics/`: imported
  converter-shaped schema mapping assignment values report unresolved paths,
  private converters, missing written import paths, arity, input type, return
  type, and purity diagnostics.
- `check/schema-reserved-bits-diagnostics/`: declaration diagnostics for
  malformed `ReservedBits(width, value)` primitive arguments.
- `check/schema-exact-width-primitive-diagnostics/`: declaration diagnostics
  when exact-width unsigned primitive names, including `UInt64be` and
  `UInt64le`, are used as ordinary source types or values.
- `run/binary-byteview-u64-helpers/`: ordinary prelude `u64` byte helpers read
  and write source-visible `Int` values in big-endian and little-endian byte
  order.
- `run/binary-byteview-u64-truncated-json/`: ordinary prelude `u64` byte reads
  use the shared byte truncation diagnostic shape.
- `run/binary-byteview-u64-write-failure-human/`: ordinary prelude `u64` byte
  writes reject values outside the source-visible unsigned `Int` boundary.
- `check/schema-ordinary-use-diagnostics/`: schema declarations do not create
  ordinary value bindings or ordinary target types for schema mappings.
- `check/codec-declarations/`: accepted private and public top-level `codec`
  declarations with explicit `decode` and `encode` directions, `derive`
  clauses, and `with` clauses.
- `check/codec-declaration-diagnostics/`: parser diagnostics for empty,
  duplicate, and unknown codec direction lists plus missing, unlisted, and
  duplicate implementation clauses.
- `check/codec-schema-references/`: same-module schema references and public
  imported schema references through written `use` paths and import aliases.
- `check/codec-schema-reference-diagnostics/`: codec schema references reject
  missing, private, wrong-kind, bare imported, missing-use, and facade
  non-reexport targets without importing schema-local fields or executable
  codec APIs.
- `check/codec-decode-signature-diagnostics/`: `decode with` checker
  diagnostics for unresolved decoder functions and wrong decode function
  parameter or return shapes.
- `check/codec-decode-signature-human/`: human `check` output keeps the
  `codec.decode_signature` primary message at the codec implementation clause
  and reports the referenced function as a related note.
- `check/codec-encode-signature-diagnostics/`: `encode with` checker
  diagnostics for unresolved encoder functions and wrong encode function
  return shapes.
- `check/codec-encode-signature-human/`: human `check` output keeps the
  `codec.encode_signature` primary message at the codec implementation clause
  and reports the referenced function as a related note.
- `check/codec-mapping-boundary/`: accepted `decode with` and `encode with`
  functions whose value type matches the referenced schema's implemented
  structural mapping target record shape.
- `check/codec-mapping-boundary-diagnostics/`: mapped `decode with` and
  `encode with` functions report `codec.decode_value_type` and
  `codec.encode_value_type` when their value boundaries do not match the
  schema mapping target record shape.
- `check/derived-codec-mapping-boundary-diagnostics/`: mapped
  `derive encode` clauses report `codec.derive_helper_unsupported` when the
  generated boundary cannot project the schema mapping target value back to
  schema-local fields.
- `check/derived-codec-helper-eligibility-diagnostics/`: unsupported
  `derive decode` and `derive encode` clauses report
  `codec.derive_helper_unsupported` with direction-specific helper details.
- `check/derived-codec-helper-eligibility-human/`: human `check` output keeps
  the derived helper eligibility primary message at the codec implementation
  clause and reports schema/helper context as related notes.
- `doc/generated-markdown/`: generated documentation from package and tool
  metadata, module identity, imports, public functions, contracts, doctest
  fences, hidden doctest setup, and ADR-lite records.
- `doc/schema-references/`: generated documentation accepts same-module public
  and private schema references plus imported public schema and schema-alias
  references.
- `doc/schema-reference-diagnostics/`: generated documentation rejects
  missing, private, wrong-kind, schema-local field, and generated helper schema
  references at the documentation reference.
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
  `format binary`, fields, field-local `where` predicates, `map to`
  arithmetic assignment expressions, and idempotence.
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
  diagnostic id, byte offset, structured field path, consumed count, and
  same-module fixture schema reference.
- `run/binary-fixture-schema-references/`: binary fixture metadata accepts
  same-module schema references, imported public schema references, and
  imported public schema-alias references with matching field paths.
- `run/binary-fixture-schema-reference-diagnostics/`: binary fixture metadata
  rejects missing, private imported, wrong-kind, generated-helper, missing-use,
  and field-path-mismatched schema references before running the command.
- `run/binary-byteview/`: `ByteView` slices, fixed-width unsigned big-endian
  and little-endian reads and writes, truncation failures, range failures, and
  conversion overflow failures, plus channel freeze preservation for bounded
  views and materialized chunks.
- `run/binary-byteview-freeze-boundary/`: channel sends and task returns
  preserve a `ByteView`'s bounded bytes, logical offset, and count after
  retained input can no longer rely on the original buffer.
- `run/binary-buffer-boundary/`: bounded `ByteView` count, take, drop, and
  slice helpers represent pending input, preserve bounded views across channel
  freeze, and construct outgoing `List<ByteChunk>` values without an
  output-only byte type.
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
  offset, field path, byte counts, readiness, and structured byte preview
  fields.
- `run/binary-schema-frame-header-reserved-json/`: frame-header reserved-bit
  validation reports `schema.reserved_bits_mismatch` through `run --json`
  with byte offset, field path, bit width, expected value, actual value, and a
  structured byte preview.
- `run/binary-schema-byte-aligned-reserved-decode/`: generated schema decode
  helpers consume byte-aligned `ReservedBits(width, value)` fields without
  exposing them in the decoded record.
- `run/binary-schema-byte-aligned-reserved-json/`: generated schema decode
  helpers report `schema.reserved_bits_mismatch` for byte-aligned reserved
  fields with field path, byte offset, bit width, expected value, actual
  value, and byte preview details.
- `run/binary-schema-byte-aligned-reserved-truncated-json/`: generated schema
  decode helpers report `schema.truncated_field` at the byte-aligned reserved
  field path when input ends before the reserved field is complete.
- `run/binary-schema-packed-reserved-decode/`: generated schema decode
  helpers consume packed `ReservedBits(width, value)` prefixes for widths one
  through seven plus visible `UIntN` fields that complete one byte and widths
  nine through fifteen plus visible `UIntN` fields that complete two
  big-endian bytes, omit the reserved fields, and continue after the shared
  storage unit.
- `run/binary-schema-packed-reserved-three-byte-decode/`: generated schema
  decode helpers consume packed `ReservedBits(width, value)` prefixes and
  suffixes that share one three-byte big-endian storage unit with visible
  `UIntN` fields, omit the reserved fields, and continue after the shared
  storage unit.
- `run/binary-schema-packed-reserved-four-byte-decode/`: generated schema
  decode helpers consume packed `ReservedBits(width, value)` prefixes and
  suffixes that share one four-byte big-endian storage unit with visible
  `UIntN` fields, omit the reserved fields, and continue after the shared
  storage unit.
- `run/binary-schema-packed-reserved-json/`: generated schema decode helpers
  report `schema.reserved_bits_mismatch` for a one-byte packed reserved
  prefix with field path, byte offset, bit width, expected value, actual
  value, and byte preview details.
- `run/binary-schema-packed-reserved-two-byte-json/`: generated schema decode
  helpers report `schema.reserved_bits_mismatch` for a two-byte packed
  reserved prefix with field path, byte offset, bit width, expected value,
  actual value, and byte preview details.
- `run/binary-schema-packed-reserved-four-byte-json/`: generated schema
  decode helpers report `schema.reserved_bits_mismatch` for a four-byte
  packed reserved prefix with field path, byte offset, bit width, expected
  value, actual value, and byte preview details.
- `run/binary-schema-middle-reserved-json/`: generated schema decode helpers
  report `schema.reserved_bits_mismatch` for a non-byte-aligned middle
  reserved field between adjacent visible `UIntN` fields, with stable field
  path, byte offset, bit width, expected value, actual value, and byte
  preview details.
- `run/binary-schema-middle-reserved-decode-encode/`: generated schema helpers
  decode and encode a non-byte-aligned middle `ReservedBits(width, value)`
  field between adjacent visible `UIntN` fields, omit the reserved field from
  the value record, preserve the adjacent visible fields, and reject an
  out-of-range adjacent visible encode value.
- `run/binary-schema-prefix-reserved-group-decode-encode/`: generated schema
  helpers decode and encode a one-byte non-byte-aligned reserved prefix group
  where `ReservedBits(width, value)` is followed by two visible `UIntN`
  fields, omit the reserved field from the value record, preserve both visible
  fields, and reject an out-of-range visible encode value.
- `run/binary-schema-prefix-reserved-two-byte-group-decode-encode/`:
  generated schema helpers decode and encode a two-byte big-endian reserved
  prefix group where `ReservedBits(width, value)` is followed by two visible
  `UIntN` fields, omit the reserved field from the value record, preserve
  both visible fields, and reject out-of-range values for either visible
  field.
- `run/binary-schema-prefix-reserved-two-byte-group-json/`: generated schema
  decode helpers report `schema.reserved_bits_mismatch` for the two-byte
  reserved prefix group with stable field path, byte offset, bit width,
  expected value, actual value, and byte preview details.
- `run/binary-schema-split-reserved-decode-encode/`: generated schema helpers
  decode and encode one shared non-byte-aligned storage byte containing more
  than one `ReservedBits(width, value)` field, omit both reserved fields from
  the value record, preserve the visible `UIntN` fields, and reject an
  out-of-range visible encode value.
- `run/binary-schema-packed-reserved-two-byte-truncated-json/`: generated
  schema decode helpers report `schema.truncated_field` at the reserved field
  path when input ends before the two-byte packed storage unit is complete.
- `run/binary-schema-packed-reserved-suffix-decode/`: generated schema decode
  helpers consume a visible one-byte `UIntN` field followed by a
  `ReservedBits(width, value)` suffix in the same byte, decode the visible
  high bits, and omit the reserved suffix field.
- `run/binary-schema-packed-reserved-suffix-json/`: generated schema decode
  helpers report `schema.reserved_bits_mismatch` for a one-byte packed
  reserved suffix with field path, byte offset, bit width, expected value, and
  actual value details.
- `run/binary-schema-packed-reserved-suffix-truncated-json/`: generated schema
  decode helpers report `schema.truncated_field` at the visible field path
  when input ends before the shared suffix storage byte is complete.
- `run/binary-schema-packed-reserved-two-byte-suffix-decode/`: generated
  schema decode helpers consume a visible `UIntN` field followed by a
  `ReservedBits(width, value)` suffix in the same two-byte big-endian storage
  unit, decode the visible high bits, and omit the reserved suffix field.
- `run/binary-schema-packed-reserved-two-byte-suffix-json/`: generated schema
  decode helpers report `schema.reserved_bits_mismatch` for a two-byte packed
  reserved suffix with field path, byte offset, bit width, expected value, and
  actual value details.
- `run/binary-schema-packed-reserved-three-byte-suffix-json/`: generated schema
  decode helpers report `schema.reserved_bits_mismatch` for a three-byte
  packed reserved suffix with field path, byte offset, bit width, expected
  value, and actual value details.
- `run/binary-schema-packed-reserved-two-byte-suffix-truncated-json/`:
  generated schema decode helpers report `schema.truncated_field` at the
  visible field path when input ends before the shared suffix storage unit is
  complete.
- `run/binary-schema-fixed-field-mismatch-json/`: generated schema decode
  helpers report `schema.fixed_field_mismatch` for a visible fixed exact-width
  field with field path, byte offset, expected value, actual value, and byte
  preview details.
- `run/binary-schema-fixed-field-mismatch-human/`: generated schema decode
  helpers report the same visible fixed exact-width field mismatch through
  human `run` output with focused primary text and related notes.
- `run/binary-schema-flag8-decode/`: generated schema decode helpers read an
  opt-in `Flag8` field as a source-visible bitset value instead of the raw
  `Int` used by `UInt8`.
- `run/binary-schema-flag8-bit-helpers/`: pure prelude helpers inspect
  decoded `Flag8` raw bits and bit positions, construct a new `Flag8` from
  raw bits and named bit indexes, and encode the result through the generated
  schema helper.
- `run/binary-schema-flag8-from-bits-out-of-range-json/`: `flag8_from_bits`
  rejects an integer outside the one-byte range with the checked runtime
  `Result` failure in JSON command output.
- `run/binary-schema-flag8-bit-index-json/`: `flag8_is_set` rejects an
  out-of-range bit index with the checked runtime `Result` failure in JSON
  command output.
- `run/binary-schema-flag8-bit-index-human/`: `flag8_set` reports the same
  out-of-range bit-index failure through human command output.
- `run/binary-schema-flag16be-bit-helpers/`: pure prelude helpers inspect
  decoded `Flag16be` raw bits and bit positions, construct a new `Flag16be`
  from raw bits and two-byte bit indexes, and encode the result through the
  generated schema helper.
- `run/binary-schema-flag16be-from-bits-out-of-range-json/`:
  `flag16be_from_bits` rejects an integer outside the two-byte range with the
  checked runtime `Result` failure in JSON command output.
- `run/binary-schema-flag16be-bit-index-json/`: `flag16be_is_set` rejects an
  out-of-range bit index with the checked runtime `Result` failure in JSON
  command output.
- `run/binary-schema-flag16be-bit-index-human/`: `flag16be_set` reports the
  same out-of-range bit-index failure through human command output.
- `run/binary-schema-flag16le-decode/`: generated schema decode helpers read a
  two-byte little-endian `Flag16le` field as a source-visible bitset value.
- `run/binary-schema-flag16le-bit-helpers/`: pure prelude helpers inspect
  decoded `Flag16le` raw bits and bit positions, construct a new `Flag16le`
  from raw bits and two-byte bit indexes, and encode the result through the
  generated schema helper.
- `run/binary-schema-flag16le-from-bits-out-of-range-json/`:
  `flag16le_from_bits` rejects an integer outside the two-byte range with the
  checked runtime `Result` failure in JSON command output.
- `run/binary-schema-flag16le-bit-index-json/`: `flag16le_is_set` rejects an
  out-of-range bit index with the checked runtime `Result` failure in JSON
  command output.
- `run/binary-schema-flag16le-bit-index-human/`: `flag16le_set` reports the
  same out-of-range bit-index failure through human command output.
- `run/binary-schema-flag32be-decode/`: generated schema decode helpers read a
  four-byte big-endian `Flag32be` field as a source-visible bitset value.
- `run/binary-schema-flag32be-bit-helpers/`: pure prelude helpers inspect
  decoded `Flag32be` raw bits and bit positions, construct a new `Flag32be`
  from raw bits and four-byte bit indexes, and encode the result through the
  generated schema helper.
- `run/binary-schema-flag32be-from-bits-out-of-range-json/`:
  `flag32be_from_bits` rejects an integer outside the four-byte range with the
  checked runtime `Result` failure in JSON command output.
- `run/binary-schema-flag32be-bit-index-json/`: `flag32be_is_set` rejects an
  out-of-range bit index with the checked runtime `Result` failure in JSON
  command output.
- `run/binary-schema-flag32be-bit-index-human/`: `flag32be_set` reports the
  same out-of-range bit-index failure through human command output.
- `run/binary-schema-flag32le-decode/`: generated schema decode helpers read a
  four-byte little-endian `Flag32le` field as a source-visible bitset value.
- `run/binary-schema-flag32le-bit-helpers/`: pure prelude helpers inspect
  decoded `Flag32le` raw bits and bit positions, construct a new `Flag32le`
  from raw bits and four-byte bit indexes, and encode the result through the
  generated schema helper.
- `run/binary-schema-flag32le-from-bits-out-of-range-json/`:
  `flag32le_from_bits` rejects an integer outside the four-byte range with the
  checked runtime `Result` failure in JSON command output.
- `run/binary-schema-flag32le-bit-index-json/`: `flag32le_is_set` rejects an
  out-of-range bit index with the checked runtime `Result` failure in JSON
  command output.
- `run/binary-schema-flag32le-bit-index-human/`: `flag32le_set` reports the
  same out-of-range bit-index failure through human command output.
- `run/binary-schema-flag64be-decode/`: generated schema decode helpers read
  an eight-byte big-endian `Flag64be` field as a source-visible bitset value.
- `run/binary-schema-flag64be-mapped-record-decode/`: direct structural decode
  mapping carries a `Flag64be` field into the mapped target record shape.
- `run/binary-schema-flag64be-bit-helpers/`: pure prelude helpers inspect
  decoded `Flag64be` raw bits and bit positions including index `63`,
  construct a new `Flag64be` from raw bits and eight-byte bit indexes, and
  encode the result through the generated schema helper.
- `run/binary-schema-flag64be-from-bits-out-of-range-json/`:
  `flag64be_from_bits` rejects an integer outside the eight-byte range with
  the checked runtime `Result` failure in JSON command output.
- `run/binary-schema-flag64be-bit-index-json/`: `flag64be_is_set` rejects an
  out-of-range bit index with the checked runtime `Result` failure in JSON
  command output.
- `run/binary-schema-flag64be-bit-index-human/`: `flag64be_set` reports the
  same out-of-range bit-index failure through human command output.
- `run/binary-schema-flag64le-decode/`: generated schema decode helpers read
  an eight-byte little-endian `Flag64le` field as a source-visible bitset
  value.
- `run/binary-schema-flag64le-mapped-record-decode/`: direct structural decode
  mapping carries a `Flag64le` field into the mapped target record shape.
- `run/binary-schema-flag64le-bit-helpers/`: pure prelude helpers inspect
  decoded `Flag64le` raw bits and bit positions including index `63`,
  construct a new `Flag64le` from raw bits and eight-byte bit indexes, and
  encode the result through the generated schema helper.
- `run/binary-schema-flag64le-from-bits-out-of-range-json/`:
  `flag64le_from_bits` rejects an integer outside the eight-byte range with
  the checked runtime `Result` failure in JSON command output.
- `run/binary-schema-flag64le-bit-index-json/`: `flag64le_is_set` rejects an
  out-of-range bit index with the checked runtime `Result` failure in JSON
  command output.
- `run/binary-schema-flag64le-bit-index-human/`: `flag64le_set` reports the
  same out-of-range bit-index failure through human command output.
- `run/binary-schema-u16le-decode/`: generated schema decode helpers read
  `UInt16le` as two little-endian bytes, return an ordinary `Int`, and keep a
  structural `map to` target record shape.
- `run/binary-schema-u16le-encode/`: generated schema encode helpers write
  `UInt16le` as two little-endian bytes in the emitted `ByteChunk`.
- `run/binary-schema-u16le-encode-out-of-range/`: generated schema encode
  helpers reject `UInt16le` values outside the unsigned 16-bit range with the
  usual `EncodeError` id, field path, and reason shape.
- `run/binary-schema-little-endian-widths-decode/`: generated schema decode
  helpers read `UInt16le`, `UInt24le`, `UInt31le`, and `UInt32le` in
  little-endian byte order, return ordinary `Int` fields, and keep a
  structural `map to` target record shape.
- `run/binary-schema-little-endian-widths-encode/`: generated schema encode
  helpers write `UInt16le`, `UInt24le`, `UInt31le`, and `UInt32le` in
  little-endian byte order in the emitted `ByteChunk`.
- `run/binary-schema-little-endian-widths-encode-out-of-range/`: generated
  schema encode helpers reject `UInt24le`, `UInt31le`, and `UInt32le` values
  outside their unsigned ranges with width-specific maximum values.
- `run/binary-schema-u31le-integer-out-of-range-json/`: generated schema
  decode helpers report `schema.integer_out_of_range` through JSON run output
  when a little-endian four-byte value exceeds the `UInt31le` range.
- `run/binary-schema-u31le-integer-out-of-range-human/`: generated schema
  decode helpers report the same `UInt31le` integer range failure through
  human `run` output with byte preview and field-path notes.
- `run/binary-schema-u48-widths-decode/`: generated schema decode helpers read
  `UInt48be` and `UInt48le` as six-byte unsigned primitive fields, preserving
  their declared byte order and structural mapping for source-visible `Int`
  values.
- `run/binary-schema-u48-widths-encode/`: generated schema encode helpers
  write `UInt48be` and `UInt48le` fields in big-endian and little-endian byte
  order.
- `run/binary-schema-u48-widths-encode-out-of-range/`: generated schema encode
  helpers reject `UInt48be` and `UInt48le` values outside the unsigned 48-bit
  range with the usual `EncodeError` id, field path, and reason shape.
- `run/binary-schema-u64-widths-decode/`: generated schema decode helpers read
  `UInt64be` and `UInt64le` as eight-byte unsigned primitive fields, preserving
  their declared byte order for source-visible `Int` values.
- `run/binary-schema-u64-widths-encode/`: generated schema encode helpers
  write `UInt64be` and `UInt64le` fields in big-endian and little-endian byte
  order.
- `run/binary-schema-u64-widths-truncated-json/`: schema decode truncation for
  a `UInt64le` field reports the shared `schema.truncated_field` JSON shape.
- `run/binary-schema-u64-widths-encode-out-of-range/`: generated schema encode
  helpers reject `UInt64be` values outside the unsigned 64-bit range with the
  usual `EncodeError` id, field path, and reason shape.
- `run/binary-schema-width-sample-decode/`: the implemented `UInt16be` and
  `UInt32be` primitive decode slice returns visible exact-width fields as
  ordinary `Int` values.
- `run/binary-schema-width-sample-truncated-json/`: schema decode truncation
  for a `UInt32be` field reports `schema.truncated_field` through JSON run
  output with byte offset, field path, byte counts, readiness, and structured
  byte preview fields.
- `run/binary-schema-repeat-decode/`: generated schema decode helpers read a
  bounded `Repeat(count_field, Primitive)` field into a `List<Int>`.
- `run/binary-schema-repeat-subtract-decode/`: generated schema decode helpers
  compute a `Repeat(left_count - right_count, Primitive)` element count from
  earlier decoded fields.
- `run/binary-schema-repeat-subtract-negative-json/`: subtraction repeat count
  decode reports `schema.length_out_of_bounds` when the computed count is
  negative.
- `run/binary-schema-repeat-truncated-json/`: repeated primitive truncation
  reports `schema.truncated_field` with the repeated field path plus an
  `index` segment for the element that could not be fully read.
- `run/binary-schema-repeat-truncated-human/`: repeated primitive truncation
  keeps the human primary message focused on the missing byte offset and puts
  readiness, byte counts, nearby bytes, and the indexed field path in notes.
- `run/binary-schema-repeat-nested-decode/`: generated schema decode helpers
  read a bounded `Repeat(count_field, SchemaName)` field into a list of nested
  decoded records.
- `run/binary-schema-repeat-nested-truncated-json/`: repeated nested schema
  truncation reports `schema.truncated_field` with the repeated field path,
  element `index`, and nested schema field path.
- `run/binary-schema-repeat-byteview-decode/`: generated schema decode helpers
  read a bounded `Repeat(count_field, ByteView(length_field))` field into a
  `List<ByteView>`.
- `run/binary-schema-repeat-byteview-truncated-json/`: repeated `ByteView`
  truncation reports `schema.truncated_field` with the repeated field path and
  failing element `index`.
- `check/binary-schema-field-reference-diagnostics/`: binary schema field
  definitions reject missing, forward, and non-`Int` schema-local references
  in repeat counts, byte-view lengths, dispatch tags, and extension-dispatch
  tags and lengths, with compatible earlier fields reported as related
  context.
- `check/binary-schema-field-reference-human/`: human `check` diagnostics for
  invalid binary schema field references keep the primary message focused on
  the failed reference and put the compatible earlier field in a note.
- `run/binary-schema-integer-out-of-range-json/`: schema decode reports
  `schema.integer_out_of_range` through JSON run output when a structurally
  present `UInt31be` field exceeds its external integer range, including byte
  offset, field path, byte width, accepted range, actual value, and structured
  byte preview fields.
- `run/binary-schema-integer-out-of-range-human/`: the same failure reports a
  focused human diagnostic with the integer range fact, nearby bytes, and
  schema field path.
- `run/binary-schema-mapped-record-decode/`: a generated binary schema decode
  helper checks field-local predicates, then maps schema-local exact-width
  fields into the target record field names before returning the decoded
  value.
- `run/binary-schema-mapped-record-expression-decode/`: a generated binary
  schema decode helper constructs a nested target record field from
  schema-local fields before returning the decoded value.
- `run/binary-schema-mapped-constructor-expression-decode/`: a generated
  binary schema decode helper constructs an ADT target field from
  schema-local fields before returning the decoded value.
- `run/binary-schema-mapped-converter-decode/`: a generated binary schema
  decode helper calls a pure same-module converter on a schema-local field
  before returning the decoded value.
- `run/binary-schema-imported-mapped-converter-decode/`: a generated binary
  schema decode helper calls imported public pure converters through written
  `use` paths before returning the decoded value.
- `run/binary-schema-mapped-byteview-decode/`: generated closed decode,
  decode-step, and derived decode codec boundaries carry a mapped
  length-bounded `ByteView` payload and preserve the consumed byte count.
- `run/binary-schema-mapped-nested-dispatch-decode/`: a generated binary
  schema decode helper maps a closed nested dispatch payload record into an
  outer target record field.
- `check/binary-schema-mixed-dispatch-selected-mapping-diagnostics/`: mixed
  dispatch payload shapes still report `schema.dispatch_payload` when selected
  mappings use a selector other than the dispatch tag field.
- `run/binary-schema-sub-byte-decode/`: generated schema decode helpers read
  standalone `UInt1` through `UInt7` visible fields from one byte each, expose
  the declared low bits as mapped `Int` values, and keep decode-step plus
  derived decode codec boundaries eligible.
- `run/binary-schema-sub-byte-decode-human/`: the same standalone sub-byte
  decode value projection is pinned in human command output.
- `run/binary-schema-sub-byte-truncated-json/`: standalone sub-byte decode
  reports `schema.truncated_field` through JSON run output when the next
  one-byte field is missing.
- `run/binary-schema-sub-byte-truncated-human/`: the same standalone
  sub-byte truncation projects the focused human diagnostic shape.
- `run/binary-schema-primitive-encode/`: a generated binary schema encode
  helper writes visible exact-width unsigned primitive `Int` fields in
  declaration order and checks complete lowercase hex output for one
  `ByteChunk`.
- `run/binary-schema-mapped-record-encode/`: the same generated encode helper
  path accepts a direct structural mapping target record and writes the
  schema-local fields projected from that target value.
- `run/binary-schema-mapped-record-expression-encode/`: generated schema
  encode helpers project a mapped target record field through a record-shaped
  mapping expression and write the recovered schema-local fields.
- `run/binary-schema-mapped-field-selection-encode/`: generated schema encode
  helpers project a mapped target field selected from a record-shaped mapping
  expression back to one schema-local field.
- `run/binary-schema-sub-byte-encode/`: generated schema encode helpers write
  standalone `UInt1` through `UInt7` visible fields as one byte each with the
  value in the declared low bits and keep derived encode codec boundaries
  eligible.
- `run/binary-schema-sub-byte-encode-human/`: the same standalone sub-byte
  encode output chunk is pinned in human command output.
- `run/binary-schema-sub-byte-encode-out-of-range/`: standalone sub-byte
  encode reports `codec.encode_value_unrepresentable` when the `Int` value
  exceeds the declared low-bit range.
- `run/binary-schema-sub-byte-encode-out-of-range-human/`: the same
  standalone sub-byte encode range failure is pinned in human command output.
- `run/binary-schema-primitive-encode-out-of-range/`: the same encode helper
  slice returns a structured `EncodeError` with
  `codec.encode_value_unrepresentable`, schema field path, and primitive range
  reason when a `UInt31be` value exceeds its maximum.
- `run/binary-schema-goaway-payload-encode/`: a schema-declared GOAWAY
  payload record encodes `ReservedBits(1, 0)`, `UInt31be`, and `UInt32be`
  through the general generated helper path, preserving field-path range
  failures for both visible payload fields.
- `run/binary-schema-flag8-encode/`: generated schema encode helpers write a
  `Flag8(bits)` field through the one-byte `UInt8` representation path.
- `run/binary-schema-flag8-encode-out-of-range/`: generated schema encode
  helpers reject `Flag8(bits)` values outside the one-byte unsigned range with
  the usual `EncodeError` id, field path, and reason shape.
- `run/binary-schema-flag8-mapped-constructor-encode/`: generated schema
  encode helpers project a single `Flag8` payload out of a mapped ADT
  constructor field and write the same one-byte representation.
- `run/binary-schema-flag8-mapped-constructor-encode-out-of-range/`: the
  mapped ADT constructor encode path preserves the ordinary `Flag8` one-byte
  range failure shape on the schema-local field path.
- `run/binary-schema-flag32be-encode/`: generated schema encode helpers write
  a `Flag32be(bits)` field through the four-byte big-endian `UInt32be`
  representation path.
- `run/binary-schema-flag32be-encode-out-of-range/`: generated schema encode
  helpers reject `Flag32be(bits)` values outside the four-byte unsigned range
  with the usual `EncodeError` id, field path, and reason shape.
- `run/binary-schema-flag32le-encode/`: generated schema encode helpers write
  a `Flag32le(bits)` field through the four-byte little-endian `UInt32le`
  representation path.
- `run/binary-schema-flag32le-encode-out-of-range/`: generated schema encode
  helpers reject `Flag32le(bits)` values outside the four-byte unsigned range
  with the usual `EncodeError` id, field path, and reason shape.
- `run/binary-schema-flag64be-encode/`: generated schema encode helpers write
  a `Flag64be(bits)` field through the eight-byte big-endian `UInt64be`
  representation path.
- `run/binary-schema-flag64be-mapped-record-encode/`: direct structural encode
  mapping projects a `Flag64be` target field back to the schema-local field.
- `run/binary-schema-flag64be-encode-out-of-range/`: generated schema encode
  helpers reject `Flag64be(bits)` values outside the eight-byte unsigned range
  with the usual `EncodeError` id, field path, and reason shape.
- `run/binary-schema-flag64le-encode/`: generated schema encode helpers write
  a `Flag64le(bits)` field through the eight-byte little-endian `UInt64le`
  representation path.
- `run/binary-schema-flag64le-mapped-record-encode/`: direct structural encode
  mapping projects a `Flag64le` target field back to the schema-local field.
- `run/binary-schema-flag64le-encode-out-of-range/`: generated schema encode
  helpers reject `Flag64le(bits)` values outside the eight-byte unsigned range
  with the usual `EncodeError` id, field path, and reason shape.
- `run/binary-schema-int-mapped-constructor-encode/`: generated schema encode
  helpers project direct single-constructor ADT integer payloads back to
  schema-local exact-width fields and write their declared byte order.
- `run/binary-schema-int-mapped-constructor-encode-out-of-range/`: the mapped
  ADT constructor encode path preserves the ordinary exact-width integer range
  failure shape on the schema-local field path.
- `run/binary-schema-multi-payload-mapped-constructor-encode/`: generated
  schema encode helpers project direct multi-payload ADT constructor values
  back to schema-local exact-width fields.
- `run/binary-schema-multi-payload-mapped-constructor-encode-mismatch/`: the
  same mapped ADT constructor encode path reports
  `codec.encode_mapping_mismatch` when the target field carries another
  constructor.
- `run/binary-schema-mapped-constructor-field-selection-encode/`: generated
  schema encode helpers project a mapped ADT constructor payload through field
  selection from a record-shaped mapping expression.
- `run/binary-schema-record-payload-mapped-constructor-encode/`: generated
  schema encode helpers destructure a mapped ADT constructor record payload
  and project its fields back to schema-local exact-width fields.
- `run/binary-schema-record-payload-mapped-constructor-encode-mismatch/`: the
  record-payload mapped ADT constructor encode path reports
  `codec.encode_mapping_mismatch` when the target field carries another
  constructor.
- `run/binary-schema-record-payload-mapped-constructor-encode-mismatch-json/`:
  the same mismatch is exposed as a run result value diagnostic with field
  path and reason details.
- `run/binary-schema-record-payload-mapped-constructor-encode-out-of-range/`:
  record-payload projection preserves the ordinary exact-width integer range
  failure shape on the projected schema-local field path.
- `run/binary-schema-repeat-encode/`: generated schema encode helpers write a
  bounded `Repeat(count_field, Primitive)` `List<Int>` field after the
  explicit count field.
- `run/binary-schema-repeat-subtract-encode/`: generated schema encode helpers
  write a bounded `Repeat(left_count - right_count, Primitive)` list field
  after the explicit count operands.
- `run/binary-schema-repeat-encode-out-of-range/`: repeated primitive encode
  rejects an element outside the selected primitive range with the usual
  `EncodeError` id, field path, and reason shape.
- `run/binary-schema-repeat-encode-count-mismatch/`: repeated primitive encode
  rejects a `List<Int>` whose length does not match the earlier count field.
- `run/binary-schema-repeat-subtract-encode-count-mismatch/`: repeated
  primitive encode rejects a `List<Int>` whose length does not match the
  computed count expression.
- `run/binary-schema-repeat-nested-encode/`: generated schema encode helpers
  write a bounded `Repeat(count_field, SchemaName)` list field by invoking the
  nested schema helper for each record.
- `run/binary-schema-repeat-nested-encode-failure/`: repeated nested schema
  encode failures prefix the nested field path with the repeated field and
  element index.
- `run/binary-schema-repeat-byteview-encode/`: generated schema encode helpers
  write a bounded `Repeat(count_field, ByteView(length_field))` field by
  emitting each element's bounded bytes in order.
- `run/binary-schema-repeat-byteview-encode-length-mismatch/`: repeated
  `ByteView` encode rejects an element whose bounded byte count does not match
  the earlier length field and reports the repeated field path plus the
  element index.
- `run/binary-schema-byteview-encode/`: generated schema encode helpers write
  the bounded bytes from a `ByteView(length_field)` payload after its explicit
  length field.
- `run/binary-schema-byteview-encode-length-mismatch/`: length-bounded
  `ByteView` encode rejects a view whose count does not match the earlier
  length field.
- `run/binary-schema-byteview-subtract-decode/`: generated schema decode
  helpers compute a `ByteView(length - padding_length)` payload count from
  earlier decoded fields.
- `run/binary-schema-byteview-subtract-negative-json/`: subtraction length
  decode reports `schema.length_out_of_bounds` when the computed length is
  negative.
- `run/binary-schema-byteview-subtract-truncated-json/`: subtraction length
  decode reports `schema.length_out_of_bounds` when the computed length
  exceeds the remaining bytes.
- `run/binary-schema-byteview-subtract-encode/`: derived schema encode accepts
  a `ByteView(length - padding_length)` payload whose view count matches the
  computed length.
- `run/binary-schema-byteview-subtract-encode-length-mismatch/`: subtraction
  length encode rejects a view whose count does not match the computed length.
- `run/binary-schema-reserved-bit-encode/`: the reserved-bit encode helper
  slice writes `ReservedBits(1, 0)` followed by `UInt31be` as one shared
  four-byte stream identifier position, omits the reserved field from the
  source value record, and checks both ordinary and maximum stream ids.
- `run/binary-schema-byte-aligned-reserved-encode/`: generated schema encode
  helpers write byte-aligned `ReservedBits(width, value)` fields from the
  declared fixed value without requiring source value record fields.
- `run/binary-schema-packed-reserved-encode/`: generated schema encode
  helpers write packed reserved prefixes for widths one through seven and
  nine through fifteen from the declared fixed value and visible low-bit
  fields from the source value record.
- `run/binary-schema-packed-reserved-three-byte-encode/`: generated schema
  encode helpers write three-byte packed reserved prefixes and suffixes from
  the declared fixed value and visible `UIntN` fields from the source value
  record.
- `run/binary-schema-packed-reserved-four-byte-encode/`: generated schema
  encode helpers write four-byte packed reserved prefixes and suffixes from
  the declared fixed value and visible `UIntN` fields from the source value
  record.
- `run/binary-schema-packed-reserved-two-byte-encode-out-of-range/`: generated
  schema encode helpers report `codec.encode_value_unrepresentable` against
  the visible low-bit field when a two-byte packed source value is outside
  its field range.
- `run/binary-schema-packed-reserved-four-byte-encode-out-of-range/`:
  generated schema encode helpers report `codec.encode_value_unrepresentable`
  against the visible low-bit field when a four-byte packed source value is
  outside its field range.
- `run/binary-schema-packed-reserved-suffix-encode/`: generated schema encode
  helpers write a visible one-byte `UIntN` field into the high bits and the
  declared reserved suffix value into the low bits.
- `run/binary-schema-packed-reserved-suffix-encode-out-of-range/`: generated
  schema encode helpers report `codec.encode_value_unrepresentable` against
  the visible high-bit field when a one-byte suffix source value is outside
  its field range.
- `run/binary-schema-packed-reserved-two-byte-suffix-encode/`: generated
  schema encode helpers write a visible `UIntN` field into the high bits and
  the declared reserved suffix value into the low bits of the same two-byte
  big-endian storage unit.
- `run/binary-schema-packed-reserved-two-byte-suffix-encode-out-of-range/`:
  generated schema encode helpers report `codec.encode_value_unrepresentable`
  against the visible high-bit field when a two-byte suffix source value is
  outside its field range.
- `run/binary-schema-middle-reserved-decode-encode/`: generated schema encode
  helpers write a middle `ReservedBits(width, value)` field between adjacent
  visible `UIntN` fields in one shared storage unit and report
  `codec.encode_value_unrepresentable` against an adjacent visible field when
  the source value is outside its bit range.
- `run/binary-schema-prefix-reserved-group-decode-encode/`: generated schema
  encode helpers write a declared reserved prefix before two visible `UIntN`
  fields in one shared byte and report `codec.encode_value_unrepresentable`
  against the visible field whose source value is outside its bit range.
- `run/binary-schema-prefix-reserved-two-byte-group-decode-encode/`:
  generated schema encode helpers write a declared reserved prefix before two
  visible `UIntN` fields in one shared two-byte big-endian storage unit and
  report `codec.encode_value_unrepresentable` against either visible field
  when the source value is outside its bit range.
- `run/binary-schema-split-reserved-decode-encode/`: generated schema encode
  helpers write multiple declared non-byte-aligned reserved fields in one
  shared storage byte with adjacent visible `UIntN` fields and report
  `codec.encode_value_unrepresentable` against an out-of-range visible field.
- `check/schema-reserved-bit-encode-diagnostics/`: valid `ReservedBits`
  syntax outside the supported reserved-bit encode layouts reports
  `schema.reserved_bits_encode` with the unsupported bit width and expected
  value.
- `run/binary-schema-closed-dispatch-encode/`: a generated binary schema
  encode helper selects a closed dispatch primitive payload from an earlier
  tag field, writes the selected big-endian or little-endian payload width,
  and returns one `ByteChunk`.
- `run/binary-schema-closed-dispatch-nested-encode/`: a generated binary
  schema encode helper selects a closed dispatch same-module nested payload
  schema and writes the nested record fields in schema order.
- `run/binary-schema-recursive-closed-dispatch-encode/`: a generated binary
  schema encode helper writes a same-module recursive closed-dispatch payload
  through a length-bounded selected mapping slice and returns one
  `ByteChunk`.
- `run/binary-schema-dispatch-nested-general-helper-encode/`: closed and
  extension-tolerant nested dispatch encode cases write the selected nested
  payload through the generated schema helper path, including byte-aligned
  reserved fields and little-endian primitive output.
- `run/binary-schema-imported-closed-dispatch-nested-encode/`: a generated
  binary schema encode helper selects a closed dispatch public imported nested
  payload schema and writes the nested record fields in schema order.
- `run/binary-schema-closed-dispatch-encode-unknown-tag/`: the same encode
  helper reports `codec.dispatch_unknown_tag` when the tag value has no
  closed dispatch case.
- `run/binary-schema-dispatch-unknown-tag-encode-diagnostic-json/`: returning
  the same `EncodeError` from `veln run --json` attaches
  `details.value_diagnostic` with field path and reason details.
- `run/binary-schema-dispatch-unknown-tag-encode-diagnostic-human/`: returning
  the same `EncodeError` from human `veln run` emits a focused runtime
  diagnostic with field path and reason notes.
- `run/binary-schema-closed-dispatch-encode-out-of-range/`: the same encode
  helper reports `codec.encode_value_unrepresentable` when the selected
  dispatch payload case is outside its primitive range.
- `run/binary-schema-encode-value-diagnostic-json/`: returning a generated
  `codec.encode_value_unrepresentable` `EncodeError` from `veln run --json`
  attaches value diagnostic details.
- `run/binary-schema-encode-value-diagnostic-human/`: returning the same
  generated encode range `EncodeError` from human `veln run` emits a focused
  runtime diagnostic with field path and reason notes.
- `run/binary-schema-encode-validation-json/`: generated schema encode
  evaluates field-local `where` predicates and reports
  `schema.validation_failed` through `run --json`.
- `run/binary-schema-mapped-encode-validation-human/`: direct mapped-record
  schema encode projects the same field-local validation failure as a focused
  human diagnostic.
- `run/binary-schema-extension-dispatch-encode/`: a generated binary schema
  encode helper writes a known extension-tolerant primitive payload selected
  by the earlier visible tag field and preserves matching unknown raw bounded
  payload bytes.
- `run/binary-schema-extension-dispatch-nested-encode/`: a generated binary
  schema encode helper writes a known extension-tolerant same-module nested
  payload schema through `SchemaDispatchPayload::Known`.
- `run/binary-schema-imported-extension-dispatch-nested-encode/`: a generated
  binary schema encode helper writes a known extension-tolerant public
  imported nested payload schema through `SchemaDispatchPayload::Known`.
- `run/binary-schema-imported-extension-dispatch-nested-encode-unknown/`: the
  same imported-case extension dispatch encode helper preserves matching
  unknown raw bounded payload bytes.
- `run/binary-schema-extension-dispatch-encode-mismatch/`: the same encode
  helper reports `codec.dispatch_mismatch` when a known tag is paired with an
  unknown payload variant.
- `run/binary-schema-dispatch-mismatch-encode-diagnostic-json/`: returning a
  generated `codec.dispatch_mismatch` `EncodeError` from `veln run --json`
  attaches value diagnostic details.
- `run/binary-schema-dispatch-mismatch-encode-diagnostic-human/`: returning
  the same dispatch mismatch `EncodeError` from human `veln run` emits a
  focused runtime diagnostic with field path and reason notes.
- `run/binary-schema-extension-dispatch-encode-tag-mismatch/`: the same
  encode helper reports `codec.dispatch_mismatch` when an unknown payload
  variant carries a tag that differs from the visible tag field.
- `run/binary-schema-extension-dispatch-encode-out-of-range/`: the same
  encode helper reports `codec.encode_value_unrepresentable` when the
  selected known primitive payload case is outside its primitive range.
- `run/binary-schema-extension-dispatch-encode-length-mismatch/`: the same
  encode helper reports `codec.dispatch_length_mismatch` when the earlier
  length field does not match the emitted payload byte count.
- `run/binary-schema-dispatch-length-encode-diagnostic-json/`: returning a
  generated `codec.dispatch_length_mismatch` `EncodeError` from
  `veln run --json` attaches value diagnostic details.
- `run/binary-schema-dispatch-length-encode-diagnostic-human/`: returning the
  same dispatch length mismatch `EncodeError` from human `veln run` emits a
  focused runtime diagnostic with field path and reason notes.
- `run/binary-schema-recursive-dispatch-length-encode-diagnostic-json/`:
  length-bounded recursive closed-dispatch encode reports
  `codec.dispatch_length_mismatch` when the supplied length field does not
  match the encoded recursive payload byte count.
- `run/binary-schema-recursive-extension-dispatch-length-encode-diagnostic-json/`:
  length-bounded recursive extension-dispatch encode reports
  `codec.dispatch_length_mismatch` when a known recursive payload byte count
  differs from the supplied length field.
- `run/binary-schema-dispatch-nested-encode-failure/`: nested payload encode
  failures report `codec.encode_value_unrepresentable` and keep the nested
  schema field path in structured `EncodeError` output.
- `run/binary-schema-imported-dispatch-nested-encode-failure/`: imported
  nested payload encode failures keep the same nested schema field path in
  structured `EncodeError` output.
- `run/binary-schema-closed-dispatch-decode/`: a generated binary schema
  decode helper reads a closed dispatch tag, selects the known payload case,
  and returns the selected payload as an ordinary `Int` field.
- `run/binary-schema-closed-dispatch-nested-decode/`: a closed dispatch known
  case may select a same-module nested binary schema payload and return the
  decoded nested record shape.
- `run/binary-schema-recursive-closed-dispatch-decode/`: a same-module
  recursive closed-dispatch payload decodes through a length-bounded closed
  dispatch, selected mappings, a non-recursive base case, and the generated
  schema helper path.
- `run/binary-schema-recursive-extension-dispatch-decode/`: a same-module
  recursive extension-dispatch payload decodes known recursive payloads through
  the generated helper path and still preserves unknown tags with bounded raw
  payload bytes.
- `run/binary-schema-dispatch-nested-general-helper-decode/`: closed and
  extension-tolerant nested dispatch known cases decode the selected nested
  payload through the generated schema helper path, preserving fixed-field
  validation, byte-aligned reserved fields, and little-endian primitive reads.
- `run/binary-schema-imported-closed-dispatch-nested-decode/`: a closed
  dispatch known case may select a public imported binary schema payload
  through a written `use` path and return the decoded nested record shape.
- `run/binary-schema-closed-dispatch-unknown-json/`: a generated binary schema
  decode helper reports `schema.dispatch_unknown_tag` through `run --json`
  with byte offset, field path, decoded tag field, decoded tag value, expected
  tags, and structured byte preview fields.
- `run/binary-schema-closed-dispatch-unknown-human/`: the same closed dispatch
  unknown-tag failure projects focused human `run` diagnostics with related
  tag, byte context, and field-path notes.
- `check/binary-schema-dispatch-payload-diagnostics/`: nested dispatch payload
  schema names are checked against the eligible generated-helper schema
  boundary, with
  diagnostics for missing names, non-schema names, private imported schemas,
  self references outside the eligible recursive length-bounded dispatch
  slice, forward references, and incompatible payload shapes.
- `check/binary-schema-recursive-dispatch-payload-diagnostics/`: recursive
  closed dispatch remains rejected when the self-reference is not
  length-bounded.
- `run/binary-schema-extension-dispatch-decode/`: a generated binary schema
  decode helper reads an extension-tolerant dispatch tag, selects a known
  payload case, and returns `SchemaDispatchPayload::Known`.
- `run/binary-schema-extension-dispatch-nested-decode/`: an
  extension-tolerant known case may decode a same-module nested binary schema
  payload and wrap the decoded record shape in
  `SchemaDispatchPayload::Known`.
- `run/binary-schema-recursive-extension-dispatch-encode/`: a same-module
  recursive extension-dispatch known payload encodes through the generated
  helper path and validates the explicit length field.
- `run/binary-schema-imported-extension-dispatch-nested-decode/`: an
  extension-tolerant known case may decode a public imported binary schema
  payload and wrap the decoded record shape in
  `SchemaDispatchPayload::Known`.
- `run/binary-schema-extension-dispatch-unknown/`: the same extension-tolerant
  dispatch form preserves an unknown tag and bounded raw payload bytes without
  reporting `schema.dispatch_unknown_tag`.
- `run/binary-schema-extension-dispatch-nested-unknown/`: unknown
  extension-tolerant tags stay opaque even when known cases name nested
  payload schemas.
- `run/binary-schema-imported-extension-dispatch-nested-unknown/`: unknown
  extension-tolerant tags stay opaque when known cases name imported nested
  payload schemas.
- `run/binary-schema-imported-dispatch-nested-failure-json/`: imported nested
  payload failures preserve the outer dispatch field path, nested schema field
  path, and absolute byte offset in JSON output.
- `run/binary-schema-dispatch-nested-failure-json/`: nested payload schema
  failures keep the outer dispatch field path, nested schema field path, and
  absolute byte offset in `run --json`.
- `run/binary-schema-dispatch-nested-general-helper-failure-json/`: fixed-field
  mismatches produced by the general nested payload helper keep the outer
  dispatch field path, nested schema field path, absolute byte offset, and byte
  preview in `run --json`.
- `run/binary-schema-recursive-dispatch-failure-json/`: recursive nested
  dispatch failures keep each outer dispatch field segment before the nested
  schema field path in `run --json`.
- `run/binary-schema-extension-dispatch-length-human/`: extension-tolerant
  dispatch still reports a focused `schema.length_out_of_bounds` diagnostic
  when the decoded unknown-payload length exceeds closed input.
- `run/binary-schema-general-helper-roundtrip/`: a non-HTTP schema combines
  `Flag8`, bounded repeat fields, representation-only reserved fields,
  `ByteView(left_length - right_length)`, same-module nested
  `ExtensionDispatch`, and little-endian nested primitive fields. The case
  checks direct helper roundtrip plus derived codec decode and encode calls
  over the same schema shape, including codec-projected `NeedMore`, decode
  `Invalid`, and encode `Invalid` outcomes.
- `run/binary-schema-decode-step/`: a generated binary schema decode-step
  helper returns `Decoded` with the exact consumed count for complete buffered
  input and `NeedMore(NeedBytes(...))` without consuming bytes for short open
  input.
- `run/codec-decode-boundary/`: a hand-written `decode with` codec item call
  passes `ByteView` and `ByteOffset` to the referenced decoder and observes
  valid `Decoded`, `NeedMore`, and `Invalid` `DecodeStep<T>` values while the
  schema mapping pins the accepted value type. It projects an oversized
  consumed count to `codec.consumed_count_invalid`.
- `run/codec-decode-consumed-count-invalid-human/` and
  `run/codec-decode-consumed-count-invalid-json/`: a hand-written codec
  boundary's stable `codec.consumed_count_invalid` decode failure projects
  through focused human diagnostics and `run --json`
  `details.byte_diagnostic` without being treated as retryable readiness.
- `run/codec-decode-invalid-step-human/` and
  `run/codec-decode-invalid-step-json/`: when a `veln run` entry returns
  `Invalid(DecodeError(...))`, `veln run` projects the contained decode error
  through focused human diagnostics and `run --json` `details.byte_diagnostic`.
- `run/codec-decode-need-more-human/` and
  `run/codec-decode-need-more-json/`: when a `veln run` entry returns
  `NeedMore(NeedBytes(...))` at a closed-input reporting boundary, `veln run`
  projects `codec.incomplete_input` through focused human diagnostics and
  `run --json` `details.byte_diagnostic`.
- `run/hpack-fixture-codec-boundary/`: an imported HPACK fixture module decodes
  deterministic header-block byte fixtures and the static indexed `0x83`
  `:method: POST`, `0x8f` `accept-charset:`, `0x90`
  `accept-encoding: gzip, deflate`, `0x91` `accept-language:`, `0x99`
  `content-disposition:`, `0x9a` `content-encoding:`, `0x9b`
  `content-language:`, `0x9c` `content-length:`, `0x9d`
  `content-location:`, `0x9e` `content-range:`, `0x9f`
  `content-type:`, `0xa0` `cookie:`, `0xa1` `date:`, `0xa2` `etag:`,
  `0xa3` `expect:`, `0xa4` `from:`, `0xa5` `host:`, `0xa6`
  `if-match:`, `0xa7` `if-modified-since:`, `0xa8`
  `if-none-match:`, `0xa9` `if-range:`, and `0xaa`
  `if-unmodified-since:`
  bytes, plus no-Huffman literal-without-indexing fixtures whose first byte
  names a supported static-table header name for `:method`, `:path`, or
  `:scheme` and whose short raw value is `PUT`, `/target`, or `https`, then
  returns ordinary header-list data and the next immutable fixture state while
  malformed literal-without-indexing input remains on the unsupported fixture
  failure path.
- `run/hpack-fixture-codec-json/` and `run/hpack-fixture-codec-human/`: an
  unsupported HPACK fixture header block projects through
  `hpack.fixture.unsupported_header_block`, separate from schema diagnostics
  and HTTP/2 frame-state diagnostics.
- `run/codec-encode-boundary/`: a hand-written `encode with` codec item call
  passes the mapped record value and ordinary encoder parameters to the
  referenced encoder and observes its returned `Encoded`, `Partial`, and
  `Invalid(EncodeError)` `EncodeStep<TState>` values unchanged. The partial
  path keeps the emitted chunk list, produced byte count, and resumed encoder
  state visible to ordinary source before resuming to a complete encode.
- `run/codec-encode-invalid-step-human/` and
  `run/codec-encode-invalid-step-json/`: when a hand-written codec encode
  entry returns `Invalid(EncodeError(...))`, `veln run` projects the contained
  encode error through focused human diagnostics and `run --json`
  `details.value_diagnostic`.
- `run/derived-codec-encode-boundary/`: a `derive encode` codec item call
  over an eligible binary schema observes successful generated helper output
  as `Encoded(List<ByteChunk>)` with one chunk and out-of-range generated
  helper failures as `Invalid(EncodeError)`.
- `run/derived-codec-mapped-encode-boundary/`: the same `derive encode`
  boundary accepts the direct structural mapping target record and projects
  one encoded output chunk.
- `run/derived-codec-selected-mapping-encode-boundary/`: the same `derive
  encode` boundary accepts a selected structural mapping target record,
  encodes both selected mapping cases, and projects helper representation
  failures to `Invalid(EncodeError)`.
- `run/derived-codec-record-payload-mapped-encode-boundary/`: the same
  `derive encode` boundary accepts a mapped target record containing an ADT
  constructor record payload and projects it to one encoded output chunk.
- `run/derived-codec-byteview-encode-boundary/`: the same `derive encode`
  boundary projects a length-bounded `ByteView` schema helper success to one
  encoded output chunk.
- `run/derived-codec-repeat-encode-boundary/`: the same `derive encode`
  boundary projects a bounded repeated primitive schema helper success to one
  encoded output chunk and helper representation failures to
  `Invalid(EncodeError)`.
- `run/derived-codec-repeat-byteview-encode-boundary/`: the same `derive
  encode` boundary projects a bounded repeated `ByteView` schema helper
  success to one encoded output chunk and helper element failures to
  `Invalid(EncodeError)`.
- `run/derived-codec-nested-dispatch-encode-boundary/`: the same `derive
  encode` codec item boundary over a same-module nested dispatch payload
  schema whose generated helper uses reserved fields and little-endian output,
  including generated helper dispatch selection failure projection.
- `run/derived-codec-imported-nested-dispatch-encode-boundary/`: the same
  `derive encode` codec item boundary over a public imported nested dispatch
  payload schema, including generated helper dispatch selection failure
  projection.
- `check/derived-codec-mapping-boundary-diagnostics/`: mapped derived encode
  clauses reject generated boundaries that cannot project the mapping target
  value back to schema-local fields.
- `check/derived-codec-helper-eligibility-diagnostics/`: derived codec
  clauses reject directions whose referenced schema cannot expose the matching
  generated helper.
- `run/derived-codec-decode-boundary/`: a `derive decode` codec item call
  over an eligible binary schema observes the generated decode-step helper's
  `Decoded`, `NeedMore`, and `Invalid` `DecodeStep<T>` values through the
  codec item name while preserving mapped record fields and no-consumption
  outcomes.
- `run/derived-codec-middle-reserved-decode-boundary/`: the same `derive
  decode` boundary observes generated decode-step helper output for a
  supported middle reserved-bit layout, including readiness and reserved-bit
  mismatch outcomes.
- `run/codec-needmore-parser-state/`: caller-owned parser state drops exactly
  the consumed prefix and advances the explicit base offset after `Decoded`,
  decodes again over the retained suffix, and keeps pending bytes plus base
  offset unchanged after `NeedMore`.
- `run/derived-codec-repeat-decode-boundary/`: the same `derive decode`
  boundary observes generated decode-step helper output for a bounded repeated
  primitive field, including readiness and helper failure outcomes.
- `run/derived-codec-repeat-byteview-decode-boundary/`: the same `derive
  decode` boundary observes generated decode-step helper output for a bounded
  repeated `ByteView` field.
- `run/derived-codec-nested-dispatch-decode-boundary/`: the same `derive
  decode` codec item boundary over a same-module nested dispatch payload
  schema whose generated helper uses field-local validation, reserved fields,
  and little-endian reads, including the generated helper's nested record
  value and consumed count.
- `run/derived-codec-imported-nested-dispatch-decode-boundary/`: the same
  `derive decode` codec item boundary over a public imported nested dispatch
  payload schema, including the generated helper's nested record value and
  consumed count.
- `run/codec-selected-mapping-decode-boundary/`: derived and hand-written
  codec decode item calls over a schema with multiple decoded-field selected
  mappings return the shared mapping target record shape.
- `check/codec-selected-mapping-boundary-diagnostics/`: hand-written codec
  decode functions over selected mappings reject the raw schema-local record
  and other wrong selected mapping value shapes.
- `run/binary-schema-frame-payload-decode/`: HTTP/2 frame decode returns the
  visible header fields plus a bounded payload `ByteView` selected by the
  decoded length.
- `run/binary-schema-frame-payload-length-json/`: a complete HTTP/2 frame
  header whose decoded length exceeds the available payload bytes reports
  `schema.length_out_of_bounds` through `run --json` with byte offset, field
  path, expected and available counts, and structured byte preview fields.
- `run/binary-schema-frame-payload-length-human/`: the same payload length
  boundary failure projects focused human `run` diagnostics with related count,
  byte context, and field-path notes.
- `run/binary-schema-validation-decode/`: field-local schema `where`
  validation preserves the decoded record shape when the predicate passes.
- `run/binary-schema-validation-json/`: field-local schema `where` validation
  failures report `schema.validation_failed` through `run --json` with byte
  offset, field path, predicate text, decoded values, and structured byte
  preview fields.
- `run/binary-schema-validation-human/`: the same validation failure projects a
  focused human `run` diagnostic with predicate, decoded-value, byte-context,
  and field-path notes.
- `run/binary-schema-validation-arithmetic-decode/`: another schema declaration
  passes field-local validation using arithmetic and boolean predicate forms.
- `run/binary-schema-validation-arithmetic-json/`: the same schema reports a
  failed arithmetic predicate through `run --json` with decoded values keyed by
  schema field name.
- `run/schema-value-validation/`: generated `validate_<schema>` accepts an
  ordinary supplied decoded schema record after field-local validation passes.
- `run/schema-value-validation-json/`: generated `validate_<schema>` reports
  failed supplied-record validation through `run --json` with
  `schema.validation_failed`, schema and field path, predicate text, supplied
  field value, and supplied decoded values.
- `run/schema-value-validation-human/`: the same supplied-record validation
  failure projects a focused human `run` diagnostic with predicate,
  supplied-value, supplied-field, and field-path notes.
- `run/binary-schema-structural-validation-decode/`: schema-level `validate`
  preserves the decoded record shape when the field relationship passes.
- `run/binary-schema-structural-validation-json/`: schema-level `validate`
  failures report `schema.validation_failed` through `run --json` with schema
  path, predicate text, decoded values, and byte preview fields.
- `run/binary-schema-structural-validation-human/`: the same schema-level
  validation failure projects a focused human `run` diagnostic with predicate,
  decoded-value, byte-context, and schema-path notes.
- `run/codec-decode-step-vocabulary/`: ordinary source constructs and matches
  `DecodeStep<T>`, `DecodeReadiness`, and `DecodeError` values for decoded,
  need-more-input, and invalid-input decoder outcomes.
- `run/codec-encode-step-vocabulary/`: ordinary source constructs and matches
  `EncodeStep<TState>` and `EncodeError` values for complete output,
  committed partial output with produced byte counts and encoder state, and
  invalid representation failures.
- `run/http2-protocol-core/`: an ordinary-source HTTP/2 sans-I/O decode state
  handles chunk arrival, client connection preface validation, incomplete
  input, end-of-stream truncation, valid CONTINUATION completion for an opaque
  header block with preserved payload bytes across multiple non-final
  CONTINUATION frames, single-frame HEADERS completion when `END_HEADERS` is
  combined with `END_STREAM`, completed HEADERS blocks that carry the HPACK
  static indexed `0x82` `:method: GET`, `0x83` `:method: POST`, `0x84`
  `:path: /`, `0x85` `:path: /index.html`, `0x86` `:scheme: http`, and
  `0x87` `:scheme: https`, plus `0x88` `:status: 200`, `0x89`
  `:status: 204`, `0x8a` `:status: 206`, `0x8b` `:status: 304`, `0x8c`
  `:status: 400`, `0x8d` `:status: 404`, `0x8e` `:status: 500`, `0x8f`
  `accept-charset:`, `0x90` `accept-encoding: gzip, deflate`, `0x91`
  `accept-language:`, `0x92` `accept-ranges:`, `0x93` `accept:`, `0x94`
  `access-control-allow-origin:`, `0x95` `age:`, `0x96` `allow:`, `0x97`
  `authorization:`, `0x98` `cache-control:`, `0x99`
  `content-disposition:`, `0x9a` `content-encoding:`, `0x9b`
  `content-language:`, `0x9c` `content-length:`, `0x9d`
  `content-location:`, `0x9e` `content-range:`, `0x9f`
  `content-type:`, `0xa0` `cookie:`, `0xa1` `date:`, `0xa2` `etag:`,
  `0xa3` `expect:`, `0xa4` `from:`, `0xa5` `host:`, `0xa6`
  `if-match:`, `0xa7` `if-modified-since:`, `0xa8`
  `if-none-match:`, `0xa9` `if-range:`, and `0xaa`
  `if-unmodified-since:`
  bytes, plus no-Huffman literal-without-indexing fixtures whose first byte
  names a supported static-table header name for `:method`, `:path`, or
  `:scheme` and whose short raw value is `PUT`, `/target`, or `https`, through
  the imported fixture codec,
  closed-by-peer stream lifecycle after accepted HEADERS `END_STREAM`
  completion through both single-frame HEADERS and final CONTINUATION paths,
  continuation ordering failures for a different frame kind
  and a different stream id, and closed input while a header block remains
  pending. It projects typed protocol
  failures, including partial and mismatched preface failures, an
  incoming frame-size peer-limit failure, a SETTINGS value range peer-limit
  failure, stream id domain failures including HEADERS and CONTINUATION on
  the connection stream, invalid stream-state frame kinds,
  wrong-length PING and GOAWAY payloads, valid PING ACK distinction,
  peer-sent `PUSH_PROMISE` rejection as a known frame kind rather than an
  unknown extension frame, and valid
  GOAWAY graceful shutdown facts plus post-GOAWAY stream rejection, into
  stable ids and related context. The
  case keeps local receive-limit
  provenance separate from peer-advertised `SETTINGS_MAX_FRAME_SIZE` state,
  keeps the local concurrent-stream receive limit separate from
  peer-advertised `SETTINGS_MAX_CONCURRENT_STREAMS` state, applies
  peer-advertised `SETTINGS_INITIAL_WINDOW_SIZE` deltas to the tracked open
  stream receive-window credit without turning that setting into an inbound
  frame-size receive limit, stores peer-advertised `SETTINGS_HEADER_TABLE_SIZE`
  and `SETTINGS_MAX_HEADER_LIST_SIZE` state with item byte offsets, and
  range-checks constrained settings before updating peer-advertised state or
  receive-window credit. Unknown SETTINGS identifiers leave peer-advertised
  state unchanged, do not produce SETTINGS range diagnostics, and do not block
  known SETTINGS items in the same frame from being applied or diagnosed. It
  also
  accepts a structurally complete
  unknown extension frame as an ordinary value preserving frame type, flags,
  stream id, and bounded payload bytes, with the preserved payload bytes also
  checked as complete lowercase hex output, while active continuation state
  still reports the existing continuation protocol failure for an unknown
  frame. The
  case also admits an idle peer-created stream on HEADERS, counts the tracked
  open peer-created stream against the active concurrent-stream receive limit,
  and reports limit exhaustion as
  `http2.peer_limit.concurrent_streams_exceeded`. It also accepts
  peer-created HEADERS at or below the recorded GOAWAY last stream id and
  rejects larger later HEADERS with `http2.protocol.stream_after_goaway`.
  The outbound HEADERS send-intent slice also accepts an open stream at the
  recorded GOAWAY boundary and rejects a higher open stream with the same
  diagnostic before frame-size or encode checks. Stream id domain failures
  and closed stream-state failures still report before the GOAWAY-specific
  check. It also accepts
  `RST_STREAM` on the tracked open stream, records the reset error code,
  clears the open stream, and rejects later DATA or stream-level
  `WINDOW_UPDATE` for that reset stream through the existing invalid
  frame-kind path. It also accepts DATA on an open stream,
  decrements both connection and stream receive-window credit by payload
  length, accepts PADDED DATA while exposing only application data bytes as
  DATA content, reports invalid DATA padding through
  `http2.protocol.invalid_data_padding`, moves that stream to closed-by-peer
  when accepted DATA carries `END_STREAM`, moves accepted HEADERS sequences
  with `END_STREAM` to the same closed-by-peer lifecycle after header-block
  completion, rejects later DATA
  and stream-level `WINDOW_UPDATE` for the closed-by-peer stream through the
  stream-state failure path, accepts
  `WINDOW_UPDATE` receive-credit increments for the connection and open
  stream, and reports zero increments, receive-window overflow,
  negative stream credit, and credit exhaustion as
  `http2.peer_limit.flow_control_window_exceeded`. After a valid non-ACK
  SETTINGS receive, it also constructs one immutable outbound SETTINGS ACK
  chunk through the frame-header encode path and shows that the send intent
  leaves peer-advertised SETTINGS state unchanged. It also constructs a local
  SETTINGS frame-header-plus-item chunk for `SETTINGS_HEADER_TABLE_SIZE`,
  `SETTINGS_INITIAL_WINDOW_SIZE`, `SETTINGS_ENABLE_PUSH`,
  `SETTINGS_MAX_CONCURRENT_STREAMS`, `SETTINGS_MAX_FRAME_SIZE`, and
  `SETTINGS_MAX_HEADER_LIST_SIZE`, records one outstanding local SETTINGS
  batch with the sent identifier and item count, clears that outstanding state
  on a valid received SETTINGS ACK, rejects local `SETTINGS_ENABLE_PUSH`
  values outside `0..1` before emitting bytes, and rejects an ACK with no
  outstanding local SETTINGS as
  `http2.protocol.unexpected_settings_ack`. After a valid inbound non-ACK
  PING frame, it constructs one immutable outbound PING ACK chunk
  through the frame-header encode path with the original opaque payload, while
  a received PING ACK remains observable and produces no response chunk. It
  also constructs outbound DATA chunks for unpadded and PADDED send-intents,
  including PADDED splitting, over-window rejection, over-frame padding
  rejection, and PADDED `END_STREAM` local closed-stream rejection of later
  DATA. It also constructs outbound PRIORITY chunks for an open stream, checks
  replacement-friendly dependency and weight values, rejects missing, closed,
  reset, mismatched, and self-dependent streams before output bytes, and
  preserves frame stream-id and dependency-payload encode failures as codec
  representation failures.
- `run/http2-protocol-core-closed-human/`: closed HTTP/2 input with undecoded
  pending bytes reports `http2.protocol.closed_with_pending` through human
  `run` stderr with byte offset, pending byte count, and active continuation
  context.
- `run/http2-protocol-core-preface-partial-human/`: end-of-stream with a
  partial client connection preface reports `http2.protocol.partial_preface`
  through human `run` stderr with pending and expected byte counts, active
  state, provenance, and a bounded lowercase hex nearby-byte note.
- `run/http2-protocol-core-preface-partial-json/`: the same partial preface
  failure reports `http2.protocol.partial_preface` through `run --json` with
  byte offset, pending byte count, expected byte count, active state, and rule
  provenance, plus structured bounded byte preview fields.
- `run/http2-protocol-core-preface-invalid-human/`: a mismatched client
  connection preface byte reports `http2.protocol.invalid_preface` through
  human `run` stderr with expected and actual byte values, matched prefix
  count, active state, provenance, and a bounded lowercase hex nearby-byte
  note.
- `run/http2-protocol-core-preface-invalid-json/`: the same invalid preface
  failure reports `http2.protocol.invalid_preface` through `run --json` with
  byte offset, expected and actual byte values, matched prefix count, expected
  byte count, active state, rule provenance, and structured bounded byte
  preview fields.
- `run/http2-protocol-core-continuation-json/`: a continuation ordering
  failure reports `http2.protocol.continuation_expected` through `run --json`
  with byte offset, frame kind, stream id, and active continuation details.
- `run/http2-protocol-core-frame-size-human/`: an incoming frame whose payload
  length exceeds the active receive maximum reports
  `http2.peer_limit.frame_size_exceeded` through human `run` stderr with a
  focused primary message and related local-configuration frame-size context.
- `run/http2-protocol-core-frame-size-json/`: the same frame-size peer-limit
  failure reports `http2.peer_limit.frame_size_exceeded` through `run --json`
  with byte offset, observed and allowed lengths, frame kind, stream
  reference, and local-configuration receive-limit provenance.
- `run/http2-protocol-core-flow-control-human/`: a DATA payload that exceeds
  available stream receive-window credit reports
  `http2.peer_limit.flow_control_window_exceeded` through human `run` stderr
  with focused window-credit, active-state, and provenance notes.
- `run/http2-protocol-core-flow-control-json/`: the same flow-control
  peer-limit failure reports
  `http2.peer_limit.flow_control_window_exceeded` through `run --json` with
  byte offset, observed payload length, allowed window credit, frame kind,
  stream reference, active state, and rule provenance.
- `run/http2-protocol-core-data-padding-human/`: invalid PADDED DATA reports
  `http2.protocol.invalid_data_padding` through human `run` stderr with pad
  length, remaining payload length, byte preview, active state, and provenance
  notes.
- `run/http2-protocol-core-data-padding-json/`: the same invalid padding
  failure reports `http2.protocol.invalid_data_padding` through `run --json`
  with byte offset, stream reference, pad length, remaining payload length,
  bounded byte preview, active state, and rule provenance.
- `run/http2-protocol-core-concurrent-streams-human/`: a peer-created stream
  that would exceed the active receive limit reports
  `http2.peer_limit.concurrent_streams_exceeded` through human `run` stderr
  with focused concurrent-stream count, state, receive-limit provenance, and
  rule provenance notes.
- `run/http2-protocol-core-concurrent-streams-json/`: the same
  concurrent-stream peer-limit failure reports
  `http2.peer_limit.concurrent_streams_exceeded` through `run --json` with
  byte offset, stream reference, attempted and allowed counts, active state,
  receive-limit provenance, and rule provenance.
- `run/http2-protocol-core-settings-value-human/`: a received
  `SETTINGS_ENABLE_PUSH` value above the accepted range reports
  `http2.peer_limit.settings_value_out_of_range` through human `run` stderr
  with the offending item byte offset, setting identity, observed value,
  accepted range, and peer-limit provenance.
- `run/http2-protocol-core-settings-value-json/`: the same SETTINGS
  peer-limit failure reports
  `http2.peer_limit.settings_value_out_of_range` through `run --json` with
  structured setting identity, observed value, accepted range, and
  peer-limit provenance fields.
- `run/http2-protocol-core-invalid-stream-id-human/`: a stream frame on
  connection stream id zero reports `http2.protocol.invalid_stream_id` through
  human `run` stderr with focused stream id domain, endpoint role, state, and
  provenance notes; the ordinary protocol-core case also covers HEADERS and
  CONTINUATION on the connection stream.
- `run/http2-protocol-core-invalid-stream-id-json/`: an even client stream id
  reports `http2.protocol.invalid_stream_id` through `run --json` with byte
  offset, frame kind, stream reference, required stream id domain, endpoint
  role, active state, and rule provenance.
- `run/http2-protocol-core-invalid-frame-kind-human/`: a DATA frame kind on
  the connection stream reports `http2.protocol.invalid_frame_kind` through
  human `run` stderr with a focused primary message and related frame-kind,
  state, and provenance notes.
- `run/http2-protocol-core-invalid-frame-kind-json/`: the same invalid
  frame-kind state failure reports `http2.protocol.invalid_frame_kind` through
  `run --json` with byte offset, actual and expected frame kinds, stream
  reference, active state, and rule provenance.
- `run/http2-protocol-core-stream-invalid-frame-kind-human/`: a DATA frame kind
  on an idle HTTP/2 stream reports `http2.protocol.invalid_frame_kind` through
  human `run` stderr with stream reference, expected frame kind, active state,
  and rule provenance notes.
- `run/http2-protocol-core-stream-invalid-frame-kind-json/`: the same
  stream-state frame-kind failure reports `http2.protocol.invalid_frame_kind`
  through `run --json` with byte offset, actual and expected frame kinds,
  stream reference, active state, and rule provenance.
- `run/http2-protocol-core-push-promise-human/`: a peer-sent `PUSH_PROMISE`
  on a nonzero stream reports `http2.protocol.invalid_frame_kind` through
  human `run` stderr with server receive state and rule provenance, confirming
  the frame kind is not preserved as an unknown extension frame.
- `run/http2-protocol-core-push-promise-json/`: the same peer-sent
  `PUSH_PROMISE` rejection reports `http2.protocol.invalid_frame_kind` through
  `run --json` with byte offset, actual frame kind, stream reference, expected
  frame kind, active state, and rule provenance.
  The focused frame-kind, stream-id, and `PUSH_PROMISE` projection cases
  declare `Http2FrameHeaderWire` locally and decode with the generated schema
  helper before projecting protocol diagnostics.
- `run/http2-protocol-core-settings-ack-length-human/`: a non-empty SETTINGS
  ACK payload reports `http2.protocol.invalid_payload_length` through human
  `run` stderr with observed and expected payload length plus protocol state
  and provenance notes.
- `run/http2-protocol-core-settings-ack-length-json/`: the same SETTINGS ACK
  payload-length failure reports `http2.protocol.invalid_payload_length`
  through `run --json` with byte offset, frame kind, stream reference,
  observed and expected payload lengths, structured byte preview, active
  state, and rule provenance.
- `run/http2-protocol-core-settings-unexpected-ack-human/`: a SETTINGS ACK
  received with no outstanding local SETTINGS reports
  `http2.protocol.unexpected_settings_ack` through human `run` stderr.
- `run/http2-protocol-core-settings-unexpected-ack-json/`: the same
  unexpected SETTINGS ACK reports `http2.protocol.unexpected_settings_ack`
  through `run --json` with byte offset, frame kind, stream reference, active
  state, and rule provenance.
- `run/http2-protocol-core-ping-length-human/`: a wrong-length PING payload
  reports `http2.protocol.invalid_payload_length` through human `run` stderr
  with observed and expected payload length, bounded inspected-payload byte
  preview, protocol state, and provenance notes.
- `run/http2-protocol-core-ping-length-json/`: the same PING payload-length
  failure reports `http2.protocol.invalid_payload_length` through `run --json`
  with byte offset, frame kind, stream reference, observed and expected
  payload lengths, structured byte preview, active state, and rule
  provenance.
- `run/http2-protocol-core-goaway-length-human/`: a wrong-length GOAWAY
  fixed-prefix payload reports `http2.protocol.invalid_payload_length` through
  human `run` stderr with observed and expected payload length plus protocol
  state and provenance notes.
- `run/http2-protocol-core-goaway-length-json/`: the same GOAWAY
  fixed-prefix payload failure reports
  `http2.protocol.invalid_payload_length` through `run --json` with byte
  offset, frame kind, stream reference, observed and expected payload lengths,
  active state, and rule provenance.
- `run/http2-protocol-core-stream-after-goaway-human/`: a peer-created
  HEADERS stream greater than a recorded GOAWAY last stream id reports
  `http2.protocol.stream_after_goaway` through human `run` stderr with
  attempted stream id, recorded last stream id, shutdown state, endpoint role,
  and rule provenance notes.
- `run/http2-protocol-core-stream-after-goaway-json/`: the same post-GOAWAY
  stream-state failure reports `http2.protocol.stream_after_goaway` through
  `run --json` with byte offset, stream reference, last stream id, shutdown
  state, endpoint role, active state, and rule provenance.
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
- `run/transport-boundary/`: descriptor-backed `net` and `time` boundary
  calls with host-fed input chunks, outgoing chunks, timeout use, and relative
  deadline waiting.
- `run/transport-socket-boundary/`: fixture-backed socket boundary calls with
  source-visible listener and stream handles, one stream read, one stream
  write, and recorded host transport events under the coarse `net` effect.
- `run/transport-socket-optional-accept-boundary/`: optional socket accept
  returns `Some(stream)` for a fixture-accepted stream, and the returned stream
  follows the existing socket read behavior.
- `run/transport-socket-optional-accept-clean-end/`: optional socket accept
  observes clean listener end as `None` without a runtime failure.
- `run/transport-socket-accept-until-boundary/`: deadline-aware socket accept
  returns `Some(stream)` when the fixture accepts before the deadline and the
  accepted stream remains usable for reads.
- `run/transport-socket-accept-until-deadline/`: deadline-aware socket accept
  observes fixture-reported accept deadline expiry as `None` without a
  runtime failure.
- `run/transport-socket-read-until-boundary/`: deadline-aware socket read
  returns `Some(bytes)` when the fixture stream yields a chunk before the
  deadline.
- `run/transport-socket-read-until-expired/`: deadline-aware socket read
  observes an already expired supplied deadline as `None` without a runtime
  failure.
- `run/transport-socket-read-until-deadline/`: deadline-aware socket read
  observes fixture-reported read deadline expiry as `None` without a runtime
  failure.
- `run/transport-socket-read-until-clean-end/`: deadline-aware socket read
  observes clean stream end as `None` without a runtime failure.
- `check/transport-socket-effects/`: listener creation, accept, stream read,
  optional clean-end listener accept and stream read, and stream write infer
  the `net` effect for public effect checking.
- `check/transport-socket-optional-accept-effects/`: optional clean-end
  listener accept directly infers the `net` effect for public effect checking.
- `check/transport-socket-accept-until-effects/`: deadline-aware listener
  accept directly infers both `net` and `time` for public effect checking.
- `check/transport-socket-read-until-effects/`: deadline-aware stream read
  directly infers both `net` and `time` for public effect checking.
- `check/transport-socket-clean-end-effects/`: optional clean-end stream read
  directly infers the `net` effect for public effect checking.
- `run/transport-deadline/`: relative deadline creation and waiting succeed
  through descriptor-backed `time` calls.
- `check/transport-deadline-effects/`: deadline creation and waiting infer
  the `time` effect for public effect checking.
- `run/transport-cancellable-wait/`: cancellable deadline creation and waiting
  succeed through descriptor-backed `time` calls and a source-visible
  `CancelToken`.
- `check/transport-cancellable-wait-effects/`: cancellable wait token
  creation and waiting infer the `time` effect for public effect checking.
- `run/transport-cancel-token-status/`: cancellation token status observation
  returns active before `time::cancel` and cancelled after it without waiting.
- `check/transport-cancel-token-status-effects/`: cancellation token status
  observation infers the `time` effect for public effect checking.
- `run/transport-cancellable-wait-outcome/`: value-returning cancellable wait
  outcomes let adapter code translate completion and cancellation into
  ordinary source decisions.
- `run/transport-cancellable-wait-outcome-deadline/`: value-returning
  cancellable wait outcomes let adapter code translate host-forced deadline
  expiry into an ordinary retry decision.
- `check/transport-cancellable-wait-outcome-effects/`: value-returning
  cancellable waits infer the `time` effect for public effect checking.
- `run/stream-adapter-cancellable-routing/`: adapter-owned stream routing
  calls a value-returning cancellable wait, routes ordinary `StreamInput`
  values through a channel, and translates completed, deadline-expired, and
  cancelled wait outcomes into ordinary response action values in one fixture
  output.
- `run/stream-adapter-cancellable-routing-deadline/`: the same adapter-owned
  routing also translates the global host-forced deadline expiry fixture into
  an ordinary retry response action value.
- `run/stream-adapter-cancellable-channel-first-routing/`: adapter-owned
  stream routing selects ordinary `StreamInput` values through receiver-list
  `channel::select_many_timeout`, then translates completed, deadline-expired,
  and cancelled wait outcomes into ordinary response action values.
- `run/channel-select-many-timeout-cancellable/`: receiver-list cancellable
  timeout selection preserves list priority for ready receivers, returns
  `Ok(None)` for timeout, and returns `Err(SelectError)` for an already
  cancelled token.
- `run/channel-select-many-timeout-cancellable-forced-cancel/`: receiver-list
  cancellable timeout selection returns `Err(SelectError)` when host-forced
  wait cancellation wins before any receiver is ready.
- `check/stream-adapter-cancellable-routing-effects/`: stream routing that
  combines cancellable waits with channels must declare both `time` and
  `concurrency`, while the pure handler boundary stays free of transport
  effects.
- `check/stream-adapter-cancellable-channel-first-routing-effects/`:
  receiver-list cancellable stream routing must declare both `time` and
  `concurrency`, while the pure handler boundary stays free of transport
  effects.
- `check/channel-select-many-timeout-cancellable-effects/`: direct
  cancellable receiver-list timeout selection must declare both `time` and
  `concurrency`.
- `run/transport-receive-malformed-json/`: malformed host-fed transport bytes
  fail as run JSON runtime errors, not schema, codec, or protocol diagnostics.
- `run/transport-send-record-failure-json/`: failed outgoing transport event
  recording fails as a run JSON runtime error.
- `run/transport-socket-read-failure-human/`: forced socket read failure stays
  runtime blame in human command output.
- `run/transport-socket-read-failure-json/`: forced socket read failure uses
  the run JSON runtime error shape.
- `run/transport-socket-read-or-end-failure-json/`: forced socket read
  failure through the optional clean-end read path uses the same run JSON
  runtime error shape.
- `run/transport-socket-optional-accept-failure-json/`: forced socket accept
  failure through the optional clean-end accept path uses the same run JSON
  runtime error shape.
- `run/transport-socket-accept-until-failure-json/`: forced socket accept
  failure through the deadline-aware optional accept path uses the same run
  JSON runtime error shape.
- `run/transport-socket-read-until-failure-json/`: forced socket read failure
  through the deadline-aware optional read path uses the same run JSON
  runtime error shape.
- `run/transport-socket-write-failure-human/`: forced socket write failure
  stays runtime blame in human command output.
- `run/transport-socket-write-failure-json/`: forced socket write failure uses
  the run JSON runtime error shape.
- `run/transport-timeout-expired-json/`: host-fixture-forced timeout expiry
  through `time::timeout_ms` fails as a run JSON runtime error.
- `run/transport-deadline-expired-human/`: host-fixture-forced deadline expiry
  through `time::wait_until` stays runtime blame in human command output.
- `run/transport-deadline-expired-json/`: host-fixture-forced deadline expiry
  through `time::wait_until` fails as a run JSON runtime error.
- `run/transport-cancellable-wait-deadline-expired-json/`: host-fixture-forced
  deadline expiry through `time::wait_until_cancellable` fails as a run JSON
  runtime error.
- `run/transport-cancellable-wait-cancelled-json/`: host-fixture-forced
  cancellation through `time::wait_until_cancellable` fails as a run JSON
  runtime error.
- `run/stream-adapter-event-boundary/`: source-owned stream event and response
  action ADTs model handler inputs and protocol intent, with direct fixture
  invocation and existing channel routing under the `concurrency` effect.
- `run/socket-stream-adapter-routing/`: multiple fixture-backed socket reads
  from one stream are wrapped as ordinary stream events, routed through a
  standard channel, handled by a plain event handler with explicit state,
  passed with explicit state, adapter context, routing metadata, and two
  additional ordinary metadata values into a six-argument spawned stream-task
  handler path, joined, and projected back to ordered socket writes by adapter
  code.
- `run/socket-stream-adapter-routing-spawn7/`: the same stream-adapter task
  boundary passes a seventh ordinary metadata value through
  `task::spawn_with7`, preserving the existing `concurrency` effect and
  socket-free handler shape.
- `run/socket-stream-adapter-routing-spawn8/`: the same stream-adapter task
  boundary passes an eighth ordinary metadata value through
  `task::spawn_with8`, preserving the existing `concurrency` effect and
  socket-free handler shape.
- `run/socket-stream-adapter-routing-spawn9/`: the same stream-adapter task
  boundary passes a ninth ordinary metadata value through
  `task::spawn_with9`, preserving the existing `concurrency` effect and
  socket-free handler shape.
- `run/socket-stream-adapter-routing-spawn10/`: the same stream-adapter task
  boundary passes a tenth ordinary metadata value through
  `task::spawn_with10`, preserving the existing `concurrency` effect and
  socket-free handler shape.
- `run/socket-stream-adapter-clean-end/`: adapter-owned source reads multiple
  socket chunks with `net::read_chunk_or_end`, observes clean end as `None`,
  translates it into `StreamInput.End`, routes stream inputs through a
  standard channel, and keeps the pure handler free of socket handles and
  `net` calls while preserving forced read failures as runtime failures.
- `run/socket-stream-adapter-owned-lifecycle/`: one adapter path owns the
  listener and accepted stream, uses `net::accept_or_end`, reads until clean
  stream end with `net::read_chunk_or_end`, routes ordinary stream values
  through a channel, invokes a pure handler, and projects `SendBytes` response
  actions to ordered `net::write_chunk` calls while keeping the same coarse
  `net` and `concurrency` effects.
- `run/socket-stream-adapter-deadline-lifecycle/`: one adapter function owns
  an accepted stream, reads deadline-aware chunks with `net::read_chunk_until`
  until a read attempt returns `None` for deadline expiry, routes ordinary
  stream values through a channel, invokes a pure handler, and projects only
  `SendBytes` response actions to ordered `net::write_chunk` calls.
- `run/channel-first-stream-routing/`: adapter-owned source routes ordinary
  `StreamInput` values through two typed channel routes, selects between them
  with existing channel selection, and then invokes a pure stream handler with
  explicit per-stream state.
- `run/channel-first-stream-routing-three-route/`: adapter-owned source routes
  ordinary `StreamInput` values through three typed channel routes, selects
  ready routes with existing channel selection, and then invokes the same pure
  stream handler shape with explicit per-stream state.
- `run/channel-first-stream-routing-four-route/`: adapter-owned source routes
  ordinary `StreamInput` values through four typed channel routes, selects
  ready routes by priority with existing channel selection, and then invokes
  the same pure stream handler shape with explicit per-stream state.
- `run/channel-first-stream-routing-five-route/`: adapter-owned source routes
  ordinary `StreamInput` values through five typed channel routes, selects the
  next ready route by priority with `channel::select_many_priority` over a
  non-empty receiver list, and then invokes the same pure stream handler shape
  with explicit per-stream state.
- `run/channel-first-stream-routing-six-route/`: adapter-owned source routes
  ordinary `StreamInput` values through six typed channel routes, selects all
  ready routes in receiver-list priority order with
  `channel::select_many_priority`, and then invokes the same pure stream
  handler shape with explicit per-stream state.
- `run/channel-first-stream-routing-seven-route/`: adapter-owned source routes
  ordinary `StreamInput` values through seven typed channel routes, selects
  all ready routes in receiver-list priority order with
  `channel::select_many_priority`, and then invokes the same pure stream
  handler shape with explicit per-stream state.
- `run/channel-first-stream-routing-eight-route/`: adapter-owned source routes
  ordinary `StreamInput` values through eight typed channel routes, selects
  all ready routes in receiver-list priority order with
  `channel::select_many_priority`, and then invokes the same pure stream
  handler shape with explicit per-stream state.
- `run/channel-first-stream-routing-nine-route/`: adapter-owned source routes
  ordinary `StreamInput` values through nine typed channel routes, selects all
  ready routes in receiver-list priority order with
  `channel::select_many_priority`, and then invokes the same pure stream
  handler shape with explicit per-stream state.
- `run/channel-select-many-timeout/`: receiver-list timeout selection keeps
  supplied receiver order as priority order, returns `None` when no receiver
  becomes ready before the timeout, returns `Ok(Some(...))` and `Ok(None)`
  through the result boundary, and routes ordinary `StreamInput` values to the
  same pure stream handler shape.
- `run/stream-adapter-cancellable-channel-first-routing/`: receiver-list
  channel-first routing can compose with value-returning cancellable waits
  while preserving ordinary handler inputs and response action values.
- `check/socket-stream-adapter-routing-effects/`: adapter-owned socket routing
  must declare the existing `net` and `concurrency` effects when it uses
  socket, channel, six-argument task spawn, and task join calls; a spawned
  handler that receives only ordinary event, state, adapter context, routing
  metadata, and two additional ordinary metadata values requires
  `concurrency` but stays free of `net` and `time`.
- `check/socket-stream-adapter-routing-spawn7-effects/`: the seven-argument
  stream-task boundary has the same effect requirements when a handler
  receives one more ordinary metadata value through `task::spawn_with7`.
- `check/socket-stream-adapter-routing-spawn8-effects/`: the eight-argument
  stream-task boundary has the same effect requirements when a handler
  receives one additional ordinary metadata value through `task::spawn_with8`.
- `check/socket-stream-adapter-routing-spawn9-effects/`: the nine-argument
  stream-task boundary has the same effect requirements when a handler
  receives one additional ordinary metadata value through `task::spawn_with9`.
- `check/socket-stream-adapter-routing-spawn10-effects/`: the ten-argument
  stream-task boundary has the same effect requirements when a handler
  receives one additional ordinary metadata value through `task::spawn_with10`.
- `check/socket-stream-adapter-owned-lifecycle-effects/`: the accepted-stream
  lifecycle shape must declare `net` and `concurrency`, while the handler
  boundary remains free of transport effects.
- `check/socket-stream-adapter-deadline-lifecycle-effects/`: the
  deadline-aware adapter lifecycle shape must declare `net`, `time`, and
  `concurrency`, while the handler boundary remains free of transport effects.
- `check/channel-first-stream-routing-effects/`: channel-first stream routing
  must declare `concurrency`, socket wrappers around that routing must declare
  both `net` and `concurrency`, and the plain handler boundary stays free of
  transport effects.
- `check/channel-first-stream-routing-three-route-effects/`: three-route
  channel-first stream routing keeps the same effect boundary as the two-route
  case: routing declares `concurrency`, socket wrappers declare `net` and
  `concurrency`, and the handler remains effect-free.
- `check/channel-first-stream-routing-four-route-effects/`: four-route
  channel-first stream routing keeps the same effect boundary as the two-route
  and three-route cases: routing declares `concurrency`, socket wrappers
  declare `net` and `concurrency`, and the handler remains effect-free.
- `check/channel-first-stream-routing-five-route-effects/`: receiver-list
  channel-first stream routing keeps the same handler boundary as the
  two-route, three-route, and four-route cases: routing declares
  `concurrency`, and the handler remains effect-free.
- `check/channel-first-stream-routing-seven-route-effects/`: seven-route
  receiver-list channel-first stream routing keeps the same handler boundary:
  routing declares `concurrency`, and the handler remains effect-free.
- `check/channel-first-stream-routing-eight-route-effects/`: eight-route
  receiver-list channel-first stream routing keeps the same handler boundary:
  routing declares `concurrency`, and the handler remains effect-free.
- `check/channel-first-stream-routing-nine-route-effects/`: nine-route
  receiver-list channel-first stream routing keeps the same handler boundary:
  routing declares `concurrency`, and the handler remains effect-free.
- `check/channel-select-many-timeout-effects/`: receiver-list timeout
  result selection keeps the same effect boundary: the routing adapter
  declares `concurrency`, and the handler remains effect-free.
- `check/stream-adapter-cancellable-channel-first-routing-effects/`:
  receiver-list channel-first routing plus value-returning cancellable waits
  requires `time` and `concurrency`, and the handler remains effect-free.
- `run/pending-input-byte-chunks/`: `StreamInput.Chunk` events append immutable
  `ByteChunk` values into bounded pending input, `End` remains distinct,
  bounded `ByteView` consumption preserves absolute `ByteOffset` facts,
  materialized consumed bytes remain readable after retained input advances,
  and protocol action values collect outgoing immutable chunks without socket
  calls.
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
