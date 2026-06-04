# Commands Full

Read [commands.md](commands.md) first unless you need command-specific
behavior, gates, or output boundaries.

## Command Sections

- [Shared command analysis](#shared-command-analysis)
- [Command help](#command-help)
- [`veln check`](#veln-check)
- [`veln fmt`](#veln-fmt)
- [`veln doc`](#veln-doc)
- [`veln run`](#veln-run)
- [`veln test`](#veln-test)
- [`veln repair`](#veln-repair)
- [`veln explain`](#veln-explain)
- [`veln package lock`](#veln-package-lock)
- [`veln lsp`](#veln-lsp)

<a id="shared-command-analysis"></a>

## Shared Command Analysis

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
selects `.veln` files below the current project root, skipping `.git` and
`target`. Explicit directories are searched recursively. The final discovered
file list is sorted and deduplicated.

If the current project root contains `veln.toml`, the command reads package
and tool metadata, path dependency entries from
`[dependencies."package"]`, git and vendor dependency metadata from the same
dependency tables, plus the implemented `[lib].exports` manifest list after
source discovery. Git dependency metadata must name a `git` remote plus
exactly one selector: `rev`, `tag`, or `branch`; `subdir` is optional
package-root metadata inside the selected source. Vendor dependency metadata
uses a string-valued `vendor` field naming an already available vendored
package directory. Current dependency discovery only reads local path
dependencies that are already available on disk; source imports do not fetch
packages, resolve git revisions, load vendored dependencies, update dependency
checksums, or write lockfiles. Current package export entries do not add files
to the selected set. Each export must be a package-relative `.veln` source
path, must use file-path spelling instead of module-path spelling, must derive
a valid source module path, must match a selected source file, and must not
duplicate another export for the same derived module path. `[modules]` is
rejected.

When a parse-clean source contains `use path from "package"`, the command
looks for a matching path dependency table in the current project manifest,
loads that dependency's discovered `.veln` sources, checks that the dependency
manifest's `[package].name` matches the requested package identity, and
requires the imported module path to be listed by the dependency package's
`[lib].exports`. The external import contributes only public declarations and
public aliases from the exported dependency module to the importing source.

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
prefix operators, binary operators, and postfix `?`.

Canonical indentation uses one tab character per indentation level. Top-level
imports, item signatures, and item-closing `end` lines use
indentation level 0. Function body lines, including contract clauses, `let`
statements, tail expressions, and standalone comments attached to those lines,
use indentation level 1.

For formatted `match` expressions, the `match` line uses the parent expression
indentation level, each arm is one indentation level deeper than that `match`
line, and the `match` closing `end` aligns with the `match` line.

Formatting accepts multiple parse-clean input files in one invocation and
writes each selected file only after all selected files have parsed without
diagnostics. The implemented golden coverage includes `ensure` clauses, prefix
and binary precedence, postfix `?`, nested records, lists, calls, and
idempotent formatting across multiple input files.

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

`doc` reads `veln.toml` when present. The implemented manifest documentation
surface accepts string-valued `[package]` fields and string-valued
`[tool.<name>]` fields. Package fields are emitted as package metadata, and
tool fields are emitted under a tool metadata section. The package `name`
field, when present, is the generated document title; otherwise the title is
`Veln Project`.

If discovery selects no source files, `doc` still emits package and tool
metadata from `veln.toml` when present. The generated module section states
that no source modules were selected.

The command has a parse gate. If any selected source has parse diagnostics, or
if manifest validation reports errors, `doc` emits human diagnostics on
stderr, writes no documentation, and exits with failure.

For `check`, `run`, `test`, and `doc`, parse-clean package-relative sources
derive local module identity from the selected `.veln` path. Path separators
become `::`. Invalid module path segments produce module diagnostics before
semantic diagnostics are reported.

For each parse-clean selected source, `doc` emits the path-derived source
module identity, the source path, imports, public source `type` declarations,
public constructors, and public `fn` declarations. Public `fn` documentation
includes attached documentation line comments and contract clauses. Public
`type` documentation includes attached documentation line comments.

Documentation line comments are attached to the nearest following module,
public type, or public function declaration only when they are immediately
above that declaration. The generated Markdown strips the `##` marker.
Executable doctest and expected-output fences remain visible examples, except
hidden setup lines whose visible doc-comment content starts with `> ` are
omitted from the generated example. ADR-lite records are emitted in a separate
ADR-lite section and keep their parsed anchor when one exists.

<a id="veln-run"></a>

## `veln run [--json] <entry> [path ...] [-- arg ...]`

`run` uses the same source discovery rule as `check`. Parse-clean files are
combined into one surface module for entry resolution. It blocks before user
code execution on parse errors, a missing entry function, an entry argument
count mismatch, an entry parameter type that cannot be supplied from command
line text, selected-entry semantic errors, reachable holes, or checked-core
blockers.

The entry must be a discovered function. Arguments after `--` are entry
arguments, not source inputs. Entry parameters may be declared as `String`,
`Int`, `Float`, or `Bool`. `String` arguments are passed through unchanged.
`Int` arguments parse as decimal signed integers, `Float` arguments parse as
JVM double-precision decimal text, and `Bool` arguments must be exactly `true`
or `false`. The reachable program is semantically checked, lowered to checked
core, then typed IR, then JVM classfile artifacts. Ordinary execution does not
write generated Java source or invoke a Java source compiler. Reachability
follows imported qualified calls by resolving the alias from selected-file `use`
declarations to the imported source module. Semantic
diagnostics in functions unreachable from the selected entry do not block
`run`.

The command caches generated JVM classfile artifacts by backend content below
the project-local build output area. On a cache miss it writes the emitted
classfiles into the cache; on a cache hit it validates the manifest and cached
classfiles before invoking `java`. Invalid or incomplete cache entries are
replaced instead of executed. Runtime trace files for JSON output remain
isolated to the individual command invocation. Human mode forwards process
stdout and stderr and returns the Java process status for runtime failures.

With `--json`, `run` captures process stdout and stderr into the run JSON
record instead of forwarding them separately. Runtime contract failures are
reported as top-level structured runtime errors with contract details.

Missing `java` before class loading is reported as a JDK setup error.

<a id="veln-test"></a>

## `veln test [--json] [target ...]`

`test` reuses the parser, semantic diagnostics, checked-core lowering, typed IR,
JVM backend, and Java execution path used by `run`, including the generated JVM
class cache.

Like `run`, `test` combines parse-clean selected files into one surface module
before semantic analysis.

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
attributes, missing `stream`, and unsupported stream values are static doc
diagnostics. When at least one output fence is present, any stream without a
fence is expected to be empty. Output comparison uses captured stdio events,
reconstructs logical stdout and stderr text, and ignores the Markdown
closing-fence newline as a raw byte assertion.

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

`package lock` reads the current project `veln.toml` and writes `veln.lock`.
The implemented package-manager slice supports dependency tables with exactly
one source field: a string-valued `path` field, a string-valued `vendor` field,
or a string-valued `git` field plus exactly one selector: `rev`, `tag`, or
`branch`. The command materializes non-local git URLs through git before
lockfile generation. It does not resolve registry sources, mirrors, or
graph-wide incompatible source selections.

For each path dependency, the dependency table key is the package identity.
The command requires the path to name an existing package root, reads that
root's `veln.toml`, and requires its `[package].name` to match the dependency
table key before writing an entry. A mismatch is reported at the dependency
table key with a related note on the dependency manifest name when available.

The written lockfile uses sorted `[[package]]` entries. Each entry records the
package `name`, a path `source` object, and a `sha256:` checksum:

```toml
[[package]]
name = "github.com/oakcask/lib"
source = { kind = "path", path = "vendor/lib" }
checksum = "sha256:..."
```

Serialized source paths use `/` separators. The checksum is computed from the
sorted `.veln` source files discovered under the dependency package root after
the same ignored-directory rule as source discovery, so `.git` and `target`
contents do not affect the lockfile.

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
change-document, and full semantic-token requests. It keeps the latest open
document text in memory and returns semantic tokens for unsaved editor content.
When a semantic-token request names a document that has not been opened through
the server, the server attempts to read the file URI from disk; unreadable
documents produce an empty token data array.

The semantic-token legend, token classes, and unsupported editor features are
specified in [editor-support.md](editor-support.md).
