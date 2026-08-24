---
role: specification
authority: normative
update-when: The CLI command behavior, command-visible output contract, command gate, source-selection contract, or command evidence route changes.
---

# Commands Full

Read [commands.md](commands.md) first unless you need command-specific
behavior, gates, or output boundaries.

## Command Sections

- [Shared command analysis](#shared-command-analysis)
- [Command help](#command-help)
- [`veln check`](#veln-check)
- [`veln fmt`](#veln-fmt)
- [`veln doc`](#veln-doc)
- [`veln metrics`](#veln-metrics)
- [`veln run`](#veln-run)
- [`veln test`](#veln-test)
- [`veln repair`](#veln-repair)
- [`veln explain`](#veln-explain)
- [`veln package lock`](#veln-package-lock)
- [`veln lsp`](#veln-lsp)
- [`veln mcp`](#veln-mcp)

<a id="shared-command-analysis"></a>

## Shared Command Analysis

Before source discovery, `check`, `doc`, `fmt`, `metrics`, `repair`, `run`,
`test`, and `package lock` resolve the invocation directory to its filesystem
identity. Each command selects the nearest ancestor with a regular
`veln.toml`. The marker is inspected without following the marker itself. A
symbolic link, directory, or other non-regular marker does not select a root.
If no ancestor qualifies, the resolved invocation directory is an anonymous
package root.

An error while classifying a marker fails the command. The command does not
continue to a wider ancestor. After a root is selected, manifest loading reads
that root's manifest. A manifest read failure fails the command and does not
trigger fallback selection.

Relative command arguments remain relative to the invocation directory. An
explicit source or test input does not select another package root. Shared
ownership validation rejects an input outside the selected package or inside a
nested package.

The checked cases `package-root-from-subdirectory` and
`package-root-relative-input` are the executable command evidence for ancestor
selection and the invocation-relative input base. The `veln-project` selector
tests cover anonymous fallback, equivalent direct and symbolic starts,
non-regular markers, classification failure, and unreadable selected
manifests. The CLI harness checks the common command entry for all listed
commands.

`check`, `run`, `test`, and `repair` use one project analysis path for source
discovery, generated doctest sources when the command includes doctests, parse
diagnostics, parse-clean surface module loading, semantic diagnostics,
checked-core readiness, and selected-entry typed-IR readiness.

Each command keeps selection, output, execution, and write policy outside that
shared path. Command-specific sections below define those user-visible
boundaries.

<a id="command-help"></a>

## Command Help

Top-level help is printed for an empty invocation, `veln --help`, `veln -h`,
and `veln help`. Subcommand help is printed for `veln help <command>` and for
`--help` or `-h` before the command-specific argument separator.

For `run`, help flags after `--` are entry arguments, not command help flags.
Unknown help topics and extra help-topic arguments are command-line errors.

Help invocations emit human help text on stdout and do not discover, parse,
check, lower, compile, run, repair, or emit command JSON.

<a id="veln-check"></a>

## `veln check [--json] [path ...]`

`check` discovers source files, parses them, combines parse-clean files into one
surface module, runs semantic diagnostics for that module, and then lowers it
far enough to report checked-core executable blockers such as missing
expressions plus call and constructor arity mismatches. With `--json`, it
prints the check JSON envelope. Without `--json`, it prints human diagnostics
or `ok`.

Inputs are files or directories. If no path is provided, discovery recursively
selects owned regular `.veln` files below the supplied project root. A regular
file named `veln.toml` in a descendant directory makes that directory a nested
package root, so discovery excludes the directory and its descendants without
opening or parsing that manifest. A symbolic link or non-regular object named
`veln.toml` is not a boundary.

Discovery does not follow source or directory symbolic links. It skips `.git`
directories. A directory named `target` is an ordinary source directory and
receives the same nested-package handling as every other directory. An error
while classifying a boundary candidate fails discovery. The final discovered
file list is sorted and deduplicated.

Explicit directories are searched recursively, but every explicit file and
directory must remain owned by the supplied project root. Discovery rejects an
input outside that root, an input below a nested manifest root, a parent-path
escape, or an input that traverses a symbolic link below the root. A nested
package rejection identifies the input and nested package root. One rejected
input fails the complete discovery operation.

The checked cases `manifest-package-boundary-discovery`,
`deep-manifest-package-boundary`, `target-owned-source-directory`,
`target-nested-package-boundary`, `anonymous-outer-package-boundary`, and
`explicit-nested-package-boundary` are the executable command evidence for
recursive and explicit boundary handling.

If the selected project root contains `veln.toml`, the command reads package
and tool metadata, path dependency entries from
`[dependencies."package"]`, git, vendor, and mirror dependency metadata from
the same dependency tables, plus the implemented `[lib].exports` manifest list
after source discovery. Git dependency metadata must name a `git` remote plus
exactly one selector: `rev`, `tag`, or `branch`; `subdir` is optional
package-root metadata inside the selected source. Vendor dependency metadata
uses a string-valued `vendor` field naming an already available vendored
package directory. Mirror dependency metadata uses a string-valued `mirror`
field naming an already materialized source tree. Current dependency discovery
loads already available direct path, vendor, mirror, and git dependency roots
for source imports. A git dependency source may be a local path, a local
`file:` URL, or a non-local URL that has already been materialized under the
project cache by another operation. When `subdir` is present, the command loads
the package root below that repository-relative subdirectory. Source imports
do not clone, fetch, check out packages, resolve git revisions, update
dependency checksums, or write lockfiles. Current package export entries do not
add files to the selected set. Each export must be a
package-relative `.veln` source path, must use file-path spelling instead of
module-path spelling, must not name a `.test.veln` test companion, must derive
a valid source module path, must match a selected source file, and must not
duplicate another export for the same derived module path. `[modules]` is
rejected.

When a parse-clean source contains `use path from "package"`, the command
looks for a matching direct path, vendor, mirror, or already available git
dependency table in the current project manifest, requires the dependency root
to have a direct regular `veln.toml`, loads that dependency's discovered
`.veln` sources, checks that the dependency manifest's `[package].name` matches
the requested package identity, and requires the imported module path to be
listed by the dependency package's
`[lib].exports`. A dependency manifest export that names a `.test.veln`
companion is rejected before that path can contribute an exported module. The
external import contributes only public declarations and public aliases from
the exported dependency module to the importing source.

The checked cases `external-package-direct-manifest` and
`external-package-missing-direct-manifest` are executable command evidence for
direct dependency package roots during source analysis. The checked cases
`external-package-imports`, `external-package-vendor-mirror-imports`, and
`external-package-git-imports` are executable command evidence for direct path,
vendor, mirror, and git import success, including git `subdir` package-root
selection. The checked cases `external-package-import-boundaries`,
`external-package-vendor-mirror-boundaries`, and
`external-package-git-boundaries` are executable command evidence for the
matching export and public visibility boundaries.

Semantic diagnostics are suppressed for a file that has parse diagnostics.
Other parse-clean files in the same invocation may still produce semantic
diagnostics. Cross-file facts from parse-clean selected files, including
source-level imports and imported qualified calls, participate in the same
semantic analysis used by `run` and `test`.

<a id="veln-fmt"></a>

## `veln fmt [path ...]`

`fmt` uses the same source discovery rule as `check`. It parses every selected
file before writing any file. If any parse diagnostic is present, the whole
format invocation exits with failure and writes nothing.

For parse-clean files, formatting is deterministic for the implemented syntax:
use declarations, function signatures, contract clauses, let statements,
tail expressions, holes with `satisfy`, records, lists, calls, literals, paths,
prefix operators, binary operators, postfix `?`, and supported binary schema
primitive compatibility spellings.

Canonical indentation uses one tab character per indentation level. Top-level
imports, item signatures, and item-closing `end` lines use
indentation level 0. Function body lines, including contract clauses, `let`
statements, tail expressions, and standalone comments attached to those lines,
use indentation level 1.

For formatted `match` expressions, the `match` line uses the parent expression
indentation level, each arm is one indentation level deeper than that `match`
line, and the `match` closing `end` aligns with the `match` line.
When a parse-clean `match` has exactly one `true` arm and one `false` arm,
`fmt` canonicalizes it to `if` / `else`; false-arm continuations that are also
ordinary `true` / `false` matches become `else if`. When a parse-clean boolean
`match` compares the same scrutinee to string, integer, float, or unit literals
through a `true` arm and a `false` continuation chain, `fmt` instead
canonicalizes it to a direct literal `match` with a wildcard fallback.
Commented rewritable matches are left in their lossless source form.

Formatting accepts multiple parse-clean input files in one invocation and
writes each selected file only after all selected files have parsed without
diagnostics. The implemented golden coverage includes `ensure` clauses, prefix
and binary precedence, postfix `?`, nested records, lists, calls, and
idempotent formatting across multiple input files. In `format binary` schemas,
supported compatibility spellings such as `UIntN`, representable
`ReservedBits(width, value)`, and `Repeat(count, Payload)` are formatted as
canonical lowercase field text, including dispatch payload field text.

Standalone line comments attach to the next parsed source line during
formatting. The formatter emits hash comments with the same indentation as the
formatted import, function signature, contract clause, body line, or closing
`end` line it documents. Comment-only lines between imports, function
signatures, contract clauses, body lines, and closing `end` lines do not
prevent parsing or deterministic formatting of those declarations. Trailing
line comments after source code stay on the same formatted source line.
`veln fmt` formats parse-clean source only; it does not migrate slash-prefixed
comment-like text.

<a id="veln-doc"></a>

## `veln doc [path ...]`

`doc` generates deterministic Markdown documentation for selected source
files. It uses the same source discovery rule as `check`: absent paths discover
`.veln` files recursively below the current project root, explicit directories
are searched recursively, and selected paths are sorted and deduplicated.
Exact `.test.veln` test companions are excluded from the generated public
document after source discovery. Explicit companion path inputs are excluded
the same way. If no non-companion source remains, `doc` still emits package and
tool metadata and the generated module section states that no source modules
were selected. `_test.veln` integration-test modules remain ordinary selected
sources for generated documentation.

`doc` reads `veln.toml` when present. The implemented manifest documentation
surface accepts string-valued `[package]` fields and string-valued
`[tool.<name>]` fields. Package fields are emitted as package metadata, and
tool fields are emitted under a tool metadata section. The package `name`
field, when present, is the generated document title; otherwise the title is
`Veln Project`.

If discovery selects no non-companion source files, `doc` still emits package
and tool metadata from `veln.toml` when present. The generated module section
states that no source modules were selected.

The command has a parse gate. If any selected source has parse diagnostics, or
if manifest validation reports errors, `doc` emits human diagnostics on
stderr, writes no documentation, and exits with failure.

For `check`, `run`, `test`, and `doc`, parse-clean package-relative sources
derive local module identity from the selected `.veln` path. Path separators
become `::`. Invalid module path segments produce module diagnostics before
semantic diagnostics are reported.

For each parse-clean selected non-companion source, `doc` emits the
path-derived source module identity, the source path, imports, public source
`type` declarations, public constructors, public `schema` declarations, public
member aliases, and public `fn` declarations. Public `fn` documentation
includes attached documentation line comments and contract clauses. Public
`type` and `schema` documentation includes attached documentation line
comments. Excluded companion sources contribute no module heading, source
path, imports, declarations, documentation comments, ADR-lite records, or
documentation schema-reference diagnostics.

Documentation line comments are attached to the nearest following module,
public type, public schema, public member alias, or public function
declaration only when they are immediately above that declaration. The
generated Markdown strips the `##` marker.
Executable doctest and expected-output fences remain visible examples, except
hidden setup lines whose visible doc-comment content starts with `> ` are
omitted from the generated example. ADR-lite records are emitted in a separate
ADR-lite section and keep their parsed anchor when one exists.
Documentation comments may write schema references as `{@schema Name}` or
`{@schema module::Name}`. The `doc` command resolves those references through
schema-aware lookup: same-module bare references may name private or public
schemas, and module-qualified references require a matching written `use` path,
including nested module paths such as `use app::nested`, and a public schema or
public schema alias. The generated Markdown renders a resolved schema reference
as code text. Missing, private, and wrong-kind schema references are name
diagnostics reported at the referenced name span. Schema-reference diagnostics
are validated for all documentation comments in selected non-companion sources,
including comments attached to private declarations that are not emitted in the
generated Markdown.

<a id="veln-metrics"></a>

## `veln metrics [--json] [--check] [--baseline PATH] [--write-baseline PATH] [path ...]`

`veln metrics` reports advisory module dependency metrics, ABC size metrics,
and experimental exact whole-body similarity for project-owned Veln source. It
follows the shared command analysis route for source discovery and parse-clean
module loading. Without `--check`, the command exits successfully when
analysis completes, even if dependency cycles, large ABC values, or duplicate
whole bodies are present.

`--json` emits the metrics JSON report specified in [metrics-json.md](metrics-json.md).
`--check` applies enabled metrics policy from `[tool.metrics]`. The current
enforceable policy is `deny_cycles = "true"`. A check with no enabled policy
is a command error.

`--write-baseline PATH` writes the current complete metrics report as
baseline schema `veln-metrics-baseline/v0` with metric model
`veln-metrics-model/v0`. The baseline records project-relative paths and does
not record absolute paths or source text. The command writes through a
temporary file in the target directory and refuses to overwrite an existing
target. `--write-baseline` conflicts with `--check` and `--json`.

`--baseline PATH` is valid only with `--check`. The baseline is loaded
explicitly from the command line; the command does not load a manifest
baseline implicitly. Unsupported baseline schema or metric model values are
comparison errors. A baseline subject that no longer exists in the current
report is reported as stale but does not by itself fail the check.

Human report output begins with summary counts, then cycles, then module rows,
then ABC size, and then `Whole-body similarity (experimental)`. Each
similarity instance names one primary declaration with its declaration and body
source locations. The remaining declarations are related locations.
Similarity output does not instruct maintainers to deduplicate code
mechanically, and similarity never creates a policy violation under `--check`.

`[tool.metrics] max_findings = "N"` limits each detailed human-output section
to its first `N` findings in canonical order. The policy-violation, cycle,
module-row, ABC-subject, and whole-body-similarity sections apply the limit
independently, so findings in one section do not hide every finding in a later
section. The default is `50`. The value must be a positive integer string
representable in the metrics JSON number domain. Zero, malformed strings,
values outside the implementation's integer range, and values outside that
JSON number domain are manifest errors at the value span. A truncated section
states its displayed, total, and omitted counts and identifies `veln metrics
--json` as the complete evidence route. The final summary states the exact
omitted count across all sections. Summary counts, section headings, policy
status, baseline status, related lines for a displayed finding, JSON arrays,
policy evaluation, and baseline content use the complete finding set.

When `deny_cycles = "true"` is checked with a baseline, a current cycle is
allowed only when its member set and cyclic edge set are subsets of one
baseline cycle. This allows unchanged cycles and cycles that lost members or
cyclic edges. New cycles, self-cycles without a matching baseline allowance,
renamed-module cycles, and cycles with added members or cyclic edges fail.

<a id="veln-run"></a>

## `veln run [--json] <entry> [path ...] [-- arg ...]`

`run` uses the same source discovery rule as `check`. Parse-clean files are
combined into one surface module for entry resolution. It blocks before user
code execution on parse errors, a missing entry function, selected-entry
semantic errors, reachable holes, an entry argument count mismatch, an entry
parameter type that cannot be supplied from command-line text, or checked-core
blockers. Reachable source-casing diagnostics for the selected entry are
reported before entry argument validation diagnostics.

The entry must be a discovered function. Arguments after `--` are entry
arguments, not source inputs. Entry parameters may be declared as `String`,
`Int`, `Float`, or `Bool`. A final variadic entry parameter may use those same
element types, and extra command-line arguments are converted to that element
type and gathered into the entry binding as `List<T>`. `String` arguments are
passed through unchanged.
`Int` arguments parse as decimal signed integers, `Float` arguments parse as
JVM double-precision decimal text, and `Bool` arguments must be exactly `true`
or `false`. Non-variadic entries keep exact argument count behavior; variadic
entries require at least the fixed parameter count. The reachable program is
semantically checked, lowered to checked core, then typed IR, then JVM
classfile artifacts. Ordinary execution does not
write generated Java source or invoke a Java source compiler. Reachability
follows imported qualified calls by resolving the alias from selected-file `use`
declarations to the imported source module. Semantic
diagnostics in functions unreachable from the selected entry do not block
`run`.

This reachability boundary includes `name.invalid_case`. An invalid covered
declaration or binding reached from the selected entry blocks `run`, while an
invalid peer outside the reachable closure does not. The checked
`identifier-casing-reachable`, `identifier-casing-unreachable`,
`identifier-casing-import-quarantine`,
`identifier-casing-imported-invalid-alias-quarantine`,
`identifier-casing-imported-invalid-type-quarantine`,
`identifier-casing-imported-invalid-payload-quarantine`,
`identifier-casing-imported-invalid-constructor-quarantine`,
`identifier-casing-invalid-entry`,
`identifier-casing-invalid-entry-json`,
`identifier-casing-invalid-entry-wrong-arity`,
`identifier-casing-invalid-entry-wrong-arity-json`,
`identifier-casing-invalid-entry-unsupported-argument-json`,
`identifier-casing-invalid-entry-conversion-json`,
`identifier-casing-alias-quarantine`, and
`identifier-casing-unused-type-alias-quarantine` run cases fix this boundary,
including first-class function value reachability that excludes quarantined
invalid function targets, invalid public function alias declaration names,
unreachable type aliases, aliases to quarantined invalid type targets, and
ADT-payload-reachable invalid public type aliases checked by
`identifier-casing-adt-payload-alias`. The
`identifier-casing-local-binding-vs-invalid-constructor`
and `identifier-casing-adt-payload-closure` cases check local binding
precedence and ADT payload closure in the same boundary.
`identifier-casing-unused-imported-invalid-type-quarantine` and
`identifier-casing-unused-imported-invalid-constructor-quarantine` check the
matching unused import boundary.
`identifier-casing-invalid-parent-constructor` checks valid constructors under
invalid parent types. The
`identifier-casing-reachable-function-alias` and
`identifier-casing-reachable-type-alias` cases check reachable invalid public
alias declaration names. Invalid public function aliases remain quarantined.
One unique same-file alias use may suppress a derivative unresolved-name
diagnostic, while independently missing alias targets still report
`name.unresolved`.
The
`identifier-casing-imported-constructor-valid-wins` and
`identifier-casing-handler-imported-constructor-valid-wins` cases check that
visible constructor resolution is not replaced by quarantined same-spelled
functions. The `identifier-casing-qualified-same-leaf` and
`identifier-casing-alias-transitive-target` cases check resolved same-leaf and
transitive public-alias targets. The
`identifier-casing-unused-handler-type-reference`,
`identifier-casing-transitive-handler-binding`,
`identifier-casing-reachable-handler-colliding-node-id`, and
`identifier-casing-underscore-type-closure` cases check handler and
underscore-led type reachability, including handler declaration identities
that collide by source-local node ordinal across files. The
`identifier-casing-mixed-json-diagnostics` case checks that `run --json`
reports a JSON diagnostic envelope when any reachable pre-execution diagnostic
is `name.invalid_case`, even when other reachable diagnostics are not casing
diagnostics. The
`unreachable-duplicate-constructor-diagnostic`,
`unreachable-type-alias-diagnostic`, and `unreachable-handler-diagnostic`
cases check that the run boundary does not hide non-casing diagnostics.

`run` and `test` cache generated JVM classfile artifacts by backend content
below the selected Veln user cache root. On Unix other than macOS, the default
root is the `veln` child of an absolute, non-empty `XDG_CACHE_HOME`, or the
`veln` child of an absolute, non-empty `HOME/.cache` fallback. On macOS, it is
the `veln` child of an absolute, non-empty `HOME/Library/Caches`. On Windows,
it is the `veln` child of an absolute, non-empty `LOCALAPPDATA`.
`VELN_CACHE_DIR`, when set, must be non-empty and lexically absolute and names
the complete Veln cache root without an added `veln` component. Selection uses
native operating-system strings and does not canonicalize or normalize the
path.

A command that needs the cache checks Java launcher availability before cache
configuration. It checks cache configuration only after successful source
analysis, executable selection, and JVM program generation. `test` checks the
configuration once before any runnable test body starts. Empty or relative
overrides do not fall back to a host base. An unavailable host base or an
unusable selected root does not fall back to the package, working directory,
`target`, or a temporary directory. Commands that do not reach JVM execution
do not inspect cache configuration.

On a cache miss the command writes the emitted classfiles into the cache; on a
cache hit it validates the manifest and cached classfiles before invoking
`java`. Invalid or incomplete cache entries are replaced instead of executed.
If an invalid entry cannot be removed, the command reports a cache error before
JVM startup and leaves the entry subject to full validation by later
invocations. If removal succeeds but preparation, prepared-entry validation,
or publication fails, the command leaves no published or partial replacement.
A later invocation observes a miss and can retry preparation and publication
below the same selected root; the failure does not select a fallback root.
When concurrent invocations prepare the same cache entry, each invocation uses
only a complete entry that validates against its own generated JVM program; an
invocation that loses publication to another writer revalidates the published
winner before using it. A writer that fails after another invocation publishes
a valid winner does not delete, replace, or invalidate that winner.
If an earlier process stops while it owns cache coordination, a later
invocation either uses a fully validated entry or reports a cache-coordination
error within an internal bound. The error occurs before JVM startup. Recovery
does not execute preparation remnants. The coordination representation,
waiting strategy, and duration are not command contracts. The fault-injected
cache evidence is in the `java::tests` unit tests. The process-level evidence
is `abandoned_jvm_cache_coordination_reaches_bounded_error_without_starting_java`
in the `toolchain_harness` test target.
Runtime trace files for command output remain isolated to the individual
command invocation. Human mode forwards process
stdout and stderr and returns the Java process status for ordinary runtime
failures. When a closed-input fixed-width `ByteView` read returns
`codec.incomplete_input`, human mode reports the missing byte at the decoded
byte offset as the primary diagnostic fact and puts pending readiness,
expected byte count, available byte count, and any available field path in
related notes. When a schema fixed-field check returns
`schema.fixed_field_mismatch`, human mode reports the fixed-field mismatch at
the decoded byte offset as the primary diagnostic fact and puts expected
value, actual value, bounded nearby byte preview, and field path in related
notes. The byte preview is rendered as lowercase hex byte pairs grouped with
spaces and includes the shown byte count, total diagnostic byte count, and
whether the preview was truncated.
When a source-visible `ByteView` range operation returns
`codec.byte_range_out_of_bounds`, human mode reports the failed range fact at
the requested byte offset and puts requested count, available count, and
bounded nearby byte preview in related notes. Checked byte write conversion
failures report `codec.byte_write_value_unrepresentable` and put the helper
name, supplied value, accepted range, width, byte order, and source-visible
`Err` value in related notes.
When binary schema frame decode returns `schema.length_out_of_bounds`, human
mode reports the failed payload boundary at the first missing byte offset and
puts expected payload count, available payload count, bounded nearby byte
preview, and field path in related notes.
When binary schema field-local validation returns `schema.validation_failed`,
human mode reports the failed validation fact at the owning field byte offset
and puts predicate text, decoded values, bounded nearby byte preview, and
field path in related notes.
When generated binary schema encode returns encode-time
`schema.validation_failed`, human mode reports the failed encode validation
fact and puts predicate text, supplied schema-local `Int` values, field path,
and the source-visible `EncodeError` value in related notes.
When a source-visible `EncodeError(...)` is returned directly from a run entry,
human mode uses the same focused encode diagnostic as the corresponding
generated encode or `EncodeStep::Invalid(EncodeError(...))` value and keeps the
rendered `EncodeError` value in related notes.
When generated length-bounded `ByteView` schema encode returns
`schema.encode_value_unrepresentable` for a count mismatch, human mode reports
the failed encode fact and puts the field path, mismatch reason, expected byte
count, actual `ByteView` count, byte offset, bounded nearby byte preview, and
the source-visible `EncodeError` value in related notes.
When binary schema decode returns `schema.integer_out_of_range`, human mode
reports the failed integer range fact at the field byte offset and puts byte
width, accepted range, actual value, bounded nearby byte preview, and field
path in related notes.
When a `veln run` entry returns
`DecodeError(id, byte_offset, field_path)`,
`DecodeErrorWithReason(id, byte_offset, field_path, reason)`,
`DecodeStep::Invalid(DecodeError(id, byte_offset, field_path))`, or
`DecodeStep::Invalid(DecodeErrorWithReason(id, byte_offset, field_path, reason))`,
human mode reports the failed decode fact at the contained byte offset and
puts field path plus the source-visible `DecodeError` value in related notes.
For `DecodeErrorWithReason`, the reason is also a related note. When an
attached reason is a byte-helper failure message with registered helper
context, human mode also puts local byte offset, expected and available byte
counts, and bounded nearby-byte preview in related notes, and `run --json`
keeps the same context in `details.byte_diagnostic`.
For `codec.checksum_mismatch`, human mode reports
`checksum mismatch at byte offset ...` and puts field path, expected checksum,
actual checksum, failure reason, and the source-visible `DecodeError` value in
related notes. `run --json` keeps the same checksum facts in
`details.byte_diagnostic.expected_checksum`, `actual_checksum`, and `reason`.
For `codec.length_mismatch`, human mode reports
`length mismatch at byte offset ...` and puts field path, expected length,
actual length, failure reason, and the source-visible `DecodeError` value in
related notes when the source-visible reason uses
`expected_length=<n>; actual_length=<n>; reason=<text>`. `run --json` keeps
the same length facts in `details.byte_diagnostic.expected_length`,
`actual_length`, and `reason`; plain reason strings keep only `reason`.
For `codec.payload_length_mismatch`, human mode reports
`payload length mismatch at byte offset ...` and puts field path, expected
payload length, actual payload length, failure reason, and the source-visible
`DecodeError` value in related notes when the source-visible reason uses
`expected_payload_length=<n>; actual_payload_length=<n>; reason=<text>`.
`run --json` keeps the same payload length facts in
`details.byte_diagnostic.expected_payload_length`, `actual_payload_length`,
and `reason`; plain reason strings keep only `reason`.
When an entry returns `DecodeStep::NeedMore(readiness)`, human mode reports
`codec.incomplete_input` at the closed-input byte boundary and puts readiness,
requested count when present, and the source-visible `DecodeStep` value in
related notes. `Decoded` entry values remain ordinary successful values.

With `--json`, `run` captures process stdout and stderr into the run JSON
record instead of forwarding them separately. Runtime contract failures are
reported as top-level structured runtime errors with contract details.

Missing `java` before class loading is reported as a JDK setup error.

<a id="veln-test"></a>

## `veln test [--json] [-j <JOBS> | --jobs <JOBS>] [target ...]`

`test` reuses the parser, semantic diagnostics, checked-core lowering, typed IR,
JVM backend, and Java execution path used by `run`, including the generated JVM
class cache.

Like `run`, `test` combines parse-clean selected files into one surface module
before semantic analysis.

`-j <JOBS>` and `--jobs <JOBS>` set the maximum number of runnable test cases
that may execute concurrently. `JOBS` is a positive decimal integer. When the
option is omitted, `test` uses the process's available parallelism and falls
back to one job if availability cannot be determined. The active worker count
is clamped to the runnable case count, and an empty runnable set starts no
workers. `--jobs 1` is the serial compatibility route for suites whose cases
share external state. The job option is recognized before `--`, including
after a target; after `--`, the same token is a target. Zero, missing,
malformed, repeated, mixed-spelling repeated, and overflowing job values are
command-line errors before project discovery.

Without explicit targets, `test` selects top-level `test` declarations in
discovered `*_test.veln` files and in any other discovered source file that
contains a top-level `test` declaration. With explicit targets, it selects
`test` declarations from the selected files, including files found recursively
below explicit directories and including non-test files. Ordinary `fn`
declarations are never selected merely because they have zero parameters.

`test` also extracts executable doctests from documentation line comments.
A doctest starts with a doc comment fence whose info string is `veln` and is
checked as a generated private `test` declaration. By default the generated
test returns `()` and declares `effects [stdio]`. A doctest fence may include
an `error=<TypePath>` info-string attribute. With that attribute, the
generated test returns `Result<(), <TypePath>>` and appends `Ok(())` as the
implicit success value, so the visible example body can use `?` without
writing harness-only success code. Without `error=<TypePath>`, a doctest that
uses `?` can infer the wrapper error type from the immediately documented
public `Result<_, E>` function or from known propagated function calls when all
of them use the same `E`. A `veln ignore` fence is documentation-only: it is
not generated, checked, selected, or paired with expected output. A `veln fail`
fence is a negative static example: `check` and `test` accept it only when its
generated source produces at least one error diagnostic; hint-only diagnostics
do not satisfy the expected failure. A negative doctest is not selected as a
runtime doctest case. A positive doctest fence may include
`runtime=contract clause=<Clause> predicate=<Predicate>` to expect a runtime
contract failure after static checking succeeds. The contract expectation
matches `require`, `ensure`, or `invariant` by contract failure kind, runtime
phase, clause, and predicate; optional `function=<Name>` and `blame=<Side>`
attributes further constrain the match. A positive doctest may instead include
`runtime=ensure predicate=<Predicate>` to expect a runtime `ensure` contract
failure after static checking succeeds, with optional `function=<Name>` and
`blame=<Side>` constraints. A positive doctest may also include
`runtime=result value=<FormattedValue>` to expect the generated test to return
`Err(<FormattedValue>)`. Other unknown executable doctest attributes, empty
metadata values, missing runtime contract `clause` or `predicate`, missing
runtime ensure `predicate`, missing runtime result `value`, and unsupported
runtime expectation kinds are static doc diagnostics. A line inside an
executable doctest fence that starts with `> ` is hidden setup: the generated
test includes the line after the marker, so the example can bind helpers
without exposing harness code in the documented sample. `# comment` lines stay
visible source comments inside generated doctests. The hidden marker is exact
after the doc-comment prefix and one optional separator space; an example that
intentionally starts source with `>` can write one extra leading space before
`>`. In `check`, generated doctests participate in parse and semantic
diagnostics. In `test`, generated positive doctests are selected as doctest
cases.

An adjacent doc comment fence whose info string is
`veln-output stream=stdout` or `veln-output stream=stderr` records expected
output for the immediately preceding executable doctest. Unknown output-fence
attributes, missing `stream`, duplicate `stream` attributes, and unsupported
stream values are static doc diagnostics. When at least one output fence is
present, any stream without a fence is expected to be empty. Output comparison
uses captured stdio events, reconstructs logical stdout and stderr text, and
ignores the Markdown closing-fence newline as a raw byte assertion.

When an explicit target names a non-test `.veln` source file, `test` also
selects a same-directory `*_test.veln` file with the same base name when that
paired file exists. The command records this in JSON output and prints a human
selection note.

For explicit non-test source targets with path-derived module identities,
`test` builds a source-level dependency graph from `use` declarations. Tests whose
transitive imports include the selected source are included in the selected
test roots before semantic analysis. If the graph is incomplete, for example
because an import has no discovered source module, `test` reports the missing
evidence and widens to all discovered tests instead of silently
under-selecting. Selected cases, static diagnostics, and JSON selection
metadata all observe the final selected target set.

Static diagnostics block the suite before Java execution. In JSON output,
already discovered cases are marked `blocked` with reason `static_gate`.
Runtime failures become failed cases. Runtime contract failures inside a
selected case use `failure.kind: "contract"` and include runtime contract
details. Tests or doctests that return `Err(value)` use
`failure.kind: "result"` and include the formatted error value. A doctest with
runtime failure metadata passes that route only when the actual runtime failure
matches the expected details. If execution succeeds or fails differently, the
case fails with
`failure.kind: "runtime_expectation"` and
`reason: "expected_runtime_failure"`. If static diagnostics block execution,
the discovered doctest case is blocked with `reason: "static_gate"`. JDK setup
failures become case errors with reason `runner_error`, including a missing
`java` before class loading.

When the static gate passes, every runnable case is scheduled through the
bounded ordered executor. The coordinator constructs and renders the report
only after all workers finish, so human status lines, JSON `cases`, diagnostics,
captured events, summary counts, failures, and exit status remain in discovered
case order regardless of completion order. Worker stdout and stderr are
captured per case and are not streamed while workers are active. The checked
examples under `../../examples/specification/test/parallel-jobs-one-json/`,
`../../examples/specification/test/parallel-jobs-two-json/`, and
`../../examples/specification/test/parallel-jobs-auto-json/` are the primary
observable specification for ordered JSON case records across serial, bounded,
and automatic job modes.

Runtime failure expectation matching is independent from expected-output
comparison. Satisfying `runtime=contract`, `runtime=ensure`, or
`runtime=result` does not satisfy any attached output fence, and matching
output does not satisfy the runtime failure expectation. The implemented
runtime expectation surface is limited to those structured contract, ensure,
and result failure kinds. Doctests do not match arbitrary panics, raw stderr
text, or process exit status as runtime failure expectations.

Doctest output mismatches become failed cases with `failure.kind: "output"` and
`reason: "expected_output"`. JSON details include the mismatched stream,
expected text, actual text, first differing logical line, bounded captured
stdio events for the actual stream, and the expected-output fence span when
available.

<a id="veln-repair"></a>

## `veln repair [--json] [--apply | --dry-run] [--candidate CANDIDATE_ID] [--confirm CANDIDATE_ID] [--override] [path ...]`

`repair` uses the same source discovery and static analysis path as `check` to
collect advisory hole repair candidates. Without `--apply`, the command is a
preview: it prints command-level repair candidates and writes no source files.
`--dry-run` is an explicit spelling of that default preview mode.

Candidate input is recomputed from the current source files unless one or more
`*.json` inputs are present. A JSON input is treated as saved repair candidate
input, not as a source file. Saved input may be a `repair --json` envelope, a
command-level candidate object or array, a `check --json` envelope, or an
advisory candidate object or array. Command-level candidate ids use the form
`repair-N` and are assigned for the current invocation. The original advisory
candidate id from diagnostic details is also preserved as
`source_candidate_id`. `--candidate` may name either id, or a saved
command-level id from a saved repair candidate, but application refuses
ambiguous ids.

Application is deliberately narrow. `--apply` applies exactly one selected
candidate; saved candidate input remains advisory rather than write
authorization. Selection, safe application, confirmation, override, target
validation, partial-application non-support, post-edit verification, and
rollback are specified in
[repair-application.md](repair-application.md).

Human preview output lists candidate ids, summaries, a representative target
span, replacement, and application policy. Human apply output reports the
applied candidate and verification result. Human refusal output starts with
`repair refused:` followed by the failed gate.

With `--json`, `repair` emits the repair JSON record described in
[repair-json.md](repair-json.md).

<a id="veln-explain"></a>

## `veln explain [--list] [diagnostic-id]`

`explain` is a read-only diagnostic catalog command. It does not discover,
parse, check, lower, compile, or run source files.

With a known diagnostic ID, it prints the diagnostic title, a short meaning,
and a repair-oriented note. With `--list`, it prints the IDs available in the
implemented catalog. Unknown IDs and an invocation without either `--list` or
a diagnostic ID are command-line errors.

The implemented catalog covers the first diagnostic families used most often
in the typed-hole and predicate repair loop:

- `hole.unfilled`
- `hole.satisfy_type_mismatch`
- `hole.satisfy_candidate_shadow`
- `hole.satisfy_candidate_unused`
- `parse.contract_predicate`
- `parse.satisfy_candidate`
- `parse.satisfy_arrow`
- `parse.satisfy_predicate`

<a id="veln-package-lock"></a>

## `veln package lock`

`package lock` reads the current project `veln.toml`, follows dependency
tables in resolved dependency manifests, and writes `veln.lock`. The
implemented package-manager slice supports dependency tables with exactly one
source field: a string-valued `path` field, a string-valued `vendor` field,
string-valued `mirror` field naming an already materialized source tree, or a
string-valued `git` field plus exactly one selector: `rev`, `tag`, or `branch`.
The command materializes non-local git URLs through git before lockfile
generation. It does not resolve registry sources.

Dependency table keys are package identities. `package lock` rejects a
dependency table key that is outside the portable package identity domain
specified by [package-snapshots.md](package-snapshots.md). A rejected key
reports `package.invalid_dependency_identity` at the dependency key and
refuses to write `veln.lock`. The checked
`../../examples/specification/package/package-lock-dot-segment-identity/` case
shows this rejection for a key with a `..` segment.

Across the graph, a package identity may resolve to only one source selection.
Repeated dependencies on the same identity are compatible when the source kind,
source location, requested git selector, and git `subdir` match after lockfile
path normalization. If a later dependency table selects a different source
location, source kind, git selector, or git `subdir` for an identity that was
already selected, `package lock` reports
`package.incompatible_dependency_source` at the later dependency key, adds a
related note for the first dependency key, and refuses to write `veln.lock`.

For each path dependency, the dependency table key is the package identity.
The command requires the path to name an existing package root, reads that
root's `veln.toml`, and requires its `[package].name` to match the dependency
table key before writing an entry. A mismatch is reported at the dependency
table key with a related note on the dependency manifest name when available.

The written lockfile uses sorted `[[package]]` entries for the resolved
dependency graph. Each entry records the package `name`, a path `source`
object, and a `sha256:` checksum:

```toml
[[package]]
name = "github.com/oakcask/lib"
source = { kind = "path", path = "vendor/lib" }
checksum = "sha256:..."
```

Serialized source paths use `/` separators. The checksum is computed from the
sorted owned `.veln` source files discovered under the dependency package root
after the same package-boundary and ignored-directory rules as source
discovery. Descendant package roots and `.git` contents do not affect the
lockfile. A directory named `target` is an ordinary source directory, so owned
`.veln` files below `target` do affect the lockfile. Lexically equivalent
dependency root spellings use the same package-relative source path names when
computing the checksum. The checked case `lock-normalized-path-dependency`
proves that a path dependency spelled with a `..` component writes the
normalized source path and computes the checksum from owned sources below that
normalized root.

For each vendor dependency, the dependency table key is the package identity
and `vendor` names an already available vendored package directory. The
command reads that directory's `veln.toml`, requires its `[package].name` to
match the dependency table key, and writes a distinct vendor source record:

```toml
[[package]]
name = "github.com/oakcask/lib"
source = { kind = "vendor", path = "vendor/lib" }
checksum = "sha256:..."
```

Vendor lockfile entries use the same source-tree checksum rule as path
dependencies. The distinct source kind preserves that the source came from
vendored package storage rather than an ordinary local path dependency.

For each mirror dependency, the dependency table key is the package identity.
The command requires `mirror` to name an already materialized package source
tree, reads that tree's `veln.toml`, and requires its `[package].name` to match
the dependency table key before writing an entry. The written mirror source
record preserves the package identity separately from the mirror source path
and checksum:

```toml
[[package]]
name = "github.com/oakcask/lib"
source = { kind = "mirror", path = "mirror/github.com/oakcask/lib" }
checksum = "sha256:..."
```

For each git dependency, the `git` value may name an already available local
repository path, a local `file:` URL, or a non-local git URL. Non-local URLs are
materialized under `.veln/package/git/` before the requested selector is
resolved. Existing materialized repositories are fetched before checkout. If
`subdir` is present, the command uses that repository-relative package root for
manifest validation and checksum generation. The dependency package root must
contain `veln.toml`, and its `[package].name` must match the dependency table
key.

The written git source record stores the package identity separately from the
source URL, requested selector, resolved commit, optional subdirectory, and
source-tree checksum:

```toml
[[package]]
name = "github.com/oakcask/lib"
source = {
  kind = "git",
  url = "vendor/mono",
  selector = { branch = "main" },
  rev = "0123456789abcdef0123456789abcdef01234567",
  subdir = "packages/lib",
}
checksum = "sha256:..."
```

<a id="veln-lsp"></a>

## `veln lsp`

`lsp` starts the editor language server over standard input and standard output
using JSON-RPC framing. It is intended for editor clients and does not take
source path arguments.

The server handles initialize, initialized, shutdown, exit, open-document,
change-document, full semantic-token, definition, prepare-rename, and rename
requests. It publishes diagnostics for open documents and for discovered
workspace sources when the client initializes workspace identity. It keeps the
latest open document text in memory and returns semantic tokens for unsaved
editor content. When a semantic-token request names a document that has not
been opened through the server, the server attempts to read the file URI from
disk; unreadable documents produce an empty token data array.

The semantic-token legend, token classes, LSP navigation support, and editor
feature boundaries are specified in [editor-support.md](editor-support.md).

<a id="veln-mcp"></a>

## `veln mcp`

`mcp` starts the agent-facing MCP server over standard input and standard
output using JSON-RPC messages. It does not take source path arguments, and it
does not run the shared package-root analysis used by `check`, `doc`, `fmt`,
`metrics`, `repair`, `run`, `test`, or `package lock`.

Standard output is reserved for MCP protocol messages. End-of-file on standard
input ends the session successfully. Startup failures are command failures
reported by the CLI command wrapper.

The MCP workspace-project selection rules, saved diagnostics, saved
definitions, implemented tools, checked tool schemas, and refresh state
transitions are specified in [mcp.md](mcp.md).
