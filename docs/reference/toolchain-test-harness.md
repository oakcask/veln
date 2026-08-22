---
role: reference
authority: normative
update-when: The CLI integration harness discovery inventory, manifest grammar, common JSON equality model, structured JSON-RPC input validation, decoded MCP JSONL output assertion model, fixture diagnostics, semantic case baseline, manifest authoring policy, case-text fixture sidecar convention, or source-error guard evidence changes.
---

# Toolchain Test Harness

This page specifies the implemented CLI integration test harness. It is a
reference for test organization, not a source for command behavior.

## Read First

- Command behavior belongs in
  [../specification/commands.md](../specification/commands.md).
- JSON output behavior belongs in
  [../specification/json-output.md](../specification/json-output.md).

## Read When

- Add a case under `toolchain_cases/` when behavior must be checked through the
  public CLI.
- Change this harness when a manifest needs a reusable assertion shape, command
  environment, repeated invocation, or fixture setup rule.
- JVM backend fixtures exercise the implemented bytecode path by default. Use
  [implemented-proposals/jvm-bytecode-backend.md](implemented-proposals/jvm-bytecode-backend.md)
  for the source-backend cleanup result.

## Case Layout

The CLI harness discovers case directories from one authoritative inventory.
The configured roots are `tests/toolchain_cases/` and
`examples/specification/`. Each descriptor records its root identity and
slash-separated case directory. Descriptor order is deterministic by the
root-qualified UTF-8 spelling, so cases with the same suffix under different
roots remain distinct.

A configured discovery root is a regular container. It must not be a symbolic
link, Windows reparse point, or other link-like entry, and it must not contain
a root-level `case.toml`. A directory with `case.toml` is a case root, but
discovery still walks below it to reject a nested `case.toml`. A nested
manifest is invalid because it would create a hidden case boundary. Discovery
also rejects symbolic links, Windows reparse points, and other link-like
entries below either root. The failure identifies the offending entry and
directs the author to use one visible, portable case boundary made of regular
fixture entries.
The preflight keeps descriptors discovered before an independent structural
error, and it still scans their manifests for policy findings. Missing roots,
links, or nested manifests do not suppress encoded line-break findings from
other reliably discovered cases. A malformed manifest also reports policy
findings collected before the syntax failure when the lexer can identify the
earlier string tokens.

The build preflight creates the generated test list from the shared inventory.
Generated cases pass a process-wide runtime barrier before manifest loading,
skip evaluation, fixture setup, or command execution. The barrier rediscovers
the current inventory and compares it with the generated list. A mismatch
fails every generated case with added or removed descriptors and directs the
maintainer to rebuild the harness.

The `runtime_inventory_barrier_*`, `toolchain_policy_preflight_*`, and
`synthetic_policy_guard_*` tests in `toolchain_harness.rs` are the executable
evidence for stale generated inventory, shared policy preflight, and
before-lifecycle ordering. The stale generated inventory test exercises the
generated-case entry path, so the production guard reaches the runtime
inventory comparison before skip evaluation, fixture copying, resource loading,
or command execution.

Cases are grouped by command or behavior area. The harness owns command
execution, fixture copying, exit-status checks, stream checks, JSON
assertions, diagnostic selectors, and file content assertions.

## Manifest Fields

- Invocation and fixture setup: `command`, `stdin`, `stdin_file`,
  `stdin_jsonrpc_file`, `repeat`, `[env]`, `[tools]`, `[requires]`, and
  `[skip]`.
- Observable command results: `exit`, `[stdout]`, `[stderr]`,
  `[help]`, `[[json_assert]]`, `[[result_value_assert]]`,
  `[[lsp_assert]]`, `[[mcp_assert]]`, `[[diagnostics]]`, `[[file_assert]]`,
  `[[binary_fixture]]`, and `[[output_chunk_list]]`.
- Manifest-failure checks: `[manifest_error]`.
- External tool setup: `[tools] java = "missing"`, `"fake-success"`, or
  `"real"` and `[tools] git = "missing"` or `"real"`.

## Manifest Value Syntax

Every scalar string field and string-array element accepts the four TOML 1.0.0
string forms: basic, literal, multiline basic, and multiline literal. Basic
strings decode the TOML escapes for backspace, tab, line feed, form feed,
carriage return, quote, backslash, and four- or eight-digit Unicode scalars.
Literal strings do not decode escapes. String tokens reject invalid Unicode
scalars and the unescaped control characters excluded by TOML 1.0.0.

The first physical newline after a multiline opener is omitted. Later physical
newlines decode to LF. A multiline basic line-ending backslash folds the next
physical lines and their surrounding whitespace. Multiline strings preserve
all other spaces and tabs. Runs of one or two delimiter quotes are content. A
closing run of three, four, or five quotes contributes zero, one, or two final
quotes. A longer run is invalid.

String-array fields accept physical newlines, comments between tokens, empty
arrays, and a trailing comma. They reject non-string elements and nested
containers. A JSON-valued `equals` field accepts physical multiline JSON arrays
and objects with nested JSON containers for `[[json_assert]]`,
`[[result_value_assert]]`, and `[[binary_fixture]]` fields that compare
structured values. JSON containers retain JSON grammar: manifest comments,
trailing commas, literal strings, and multiline TOML strings are invalid. A
complete manifest string token used as `equals` becomes a JSON string value.
JSON string escapes decode UTF-16 surrogate pairs to one scalar value and
reject unpaired high or low surrogates.

LF and CRLF each count as one physical newline. Physical newlines inside
multiline strings normalize to LF, including mixed-line-ending manifests. A
lone CR is invalid. Manifest failures report a one-based physical line. Local
token and JSON grammar failures take precedence over missing outer manifest
delimiters, including incomplete JSON object members, array elements, escapes,
and strings. If clean end of input leaves only delimiters missing, the failure
points to the innermost unmatched opener. A fixed-width Unicode escape that is
cut short by end of input, LF, or CRLF fails on the line where that escape
begins.

The table-driven `manifest_*` tests in `toolchain_harness.rs` are the
executable evidence for string forms, escapes, folding, quote runs,
indentation and closing delimiters, physical-newline equivalence, schema
selected string arrays, field-directed containers, trailing tokens, and exact
error lines. The checked semantic baseline and the complete harness target
protect existing single-line case meaning.

The manifest policy scanner uses the same lexer provenance. It examines every
TOML string token and every string token inside JSON-valued manifest fields,
including nested JSON object keys, object values, and array values. JSON string
tokens use JSON escape semantics, so valid JSON escapes such as `\/` do not
fail the policy scan, while invalid JSON string escapes are retained as
manifest policy scan failures. The predicate rejects a decoded LF or CR that
comes from an escape, and it rejects decoded text that spells a line-break
escape such as `\n`, `\r`, `\u000A`, `\u000D`, `\U0000000A`, or
`\U0000000D`. Physical newlines inside multiline strings are valid. Comments,
separate string tokens, non-string values, and sidecar files are outside this
predicate.

Policy findings are sorted by root-qualified path, source line, token span,
and category. A finding identifies the manifest field, location, offending
spelling, the replacement action, and the reviewability reason: line structure
belongs in physical multiline text or a sidecar so fixture changes remain
visible. If manifest lexing stops at a boundary error such as an unterminated
string or lone carriage return, the policy scan still reports findings from
completed earlier statements before it reports the manifest syntax failure.
The preflight aggregates those retained findings with skipped-case,
unavailable-tool, and malformed-manifest failures in deterministic
root-qualified order. The summary reports both the total number of problems
and the number of distinct affected manifest descriptors.

## Output Cases

Use `exit`, `[stdout]`, and `[stderr]` for command-visible output. Stream
sections accept `format = "empty"`, `"text"`, or `"json"` where JSON is valid
for stdout, plus `contains`, `contains_file`, `contains_files`,
`not_contains`, `not_contains_file`, and `not_contains_files` fragments for
stable text checks. Fragment fields append in manifest order and may repeat
when a file-backed fragment belongs between inline fragments. Stream exact
equality uses `equals_file`; the harness reads the expected text from the
discovered case before the command runs.

Use `stdin_file` when command input is easier to review as a case text file.
Use a `.raw` sidecar for `stdin_file` when the input protocol includes bytes
whose framing must survive repository checkout exactly, such as LSP JSON-RPC
headers and their CRLF separators.
MCP cases use `stdin_file` with stream fragments or `equals_file` sidecars when
the behavior under test is the newline-delimited stdio protocol itself. Keep
those fixtures as ordinary `case-text/` files when LF-normalized JSON lines are
the intended observable bytes.

Use repeatable `[[mcp_assert]]` sections to check decoded newline-delimited
JSON-RPC responses from `veln mcp` stdout. Each nonempty stdout line must
decode as one JSON object. Malformed JSON and non-object lines fail decoded MCP
assertions before any response-local assertion runs. Each assertion selects
exactly one response by `id`, where the manifest value must be a JSON string or
syntactically integer JSON number. Integer selector IDs are accepted even when
their token is outside the harness `i64` storage range. Non-integer decimal and
exponent number tokens are not selector IDs. A missing selected ID fails. More
than one response with the selected ID fails. Other response IDs can be present
in the same stream.

Each MCP assertion declares `path` as an RFC 6901 JSON Pointer. The empty
pointer selects the complete response. A nonempty pointer must start with `/`.
The pointer escape sequences `~0` and `~1` decode to `~` and `/`. A missing
path can satisfy only `missing = true`; invalid traversal through a scalar or
noncanonical array index fails.

Each MCP assertion declares exactly one of `equals`, `length`,
`workspace_file_uri`, or `missing = true`. `equals` uses the common JSON
equality rules below. `length` requires a JSON array at the selected path and
checks its exact element count.
`workspace_file_uri` requires a JSON string at the selected path and compares
it with the canonical `file:` URI for one existing regular
workspace-relative file in the copied case project. The URI spelling matches
the `definition` MCP producer, including percent-encoding native non-Unix path
separators instead of normalizing them to `/`. The operand rejects absolute
paths, empty paths, `.`, `..`, empty segments, backslashes, symbolic links,
Windows reparse points and other link-like path components, non-file entries,
and canonical paths that leave the workspace root.

Use `stdin_jsonrpc_file` for an ordered UTF-8 JSON array of JSON-RPC requests
and notifications. It is mutually exclusive with `stdin` and `stdin_file`.
Each array element must be an object. Its `jsonrpc` member must be `"2.0"`.
Its `method` member must be a string. An `id` may be absent, a string, a
number, or null. A `params` member may be absent, an object, an array, or null.
The harness rejects response-shaped input containing `result` or `error`. It
does not reject unknown methods, extension members, duplicate identifiers, or
method-specific parameter values. The standard envelope members `jsonrpc`,
`method`, `id`, and `params` must not appear more than once in the same
message object, including when one duplicate spelling has an otherwise valid
value and another duplicate spelling has an invalid value. JSON string escapes
in the request file decode paired UTF-16 surrogates before framing and reject
unpaired surrogate units before command startup.

Any complete object value or array element in a structured request may use a
`{"$case_text":"relative/path"}` directive. The object must contain only that
member, and its value must be a string. The harness recursively replaces the
directive with the exact UTF-8 snapshot supplied by the case-relative file
rules below. Object member order, array element order, a byte-order mark,
CRLF, non-ASCII text, and a final line break remain observable through the
serialized string value.

The harness serializes each expanded message as compact deterministic JSON. It
prefixes the body with `Content-Length: <UTF-8 byte length>\r\n\r\n` and
concatenates frames in array order. Malformed JSON, root and element kind
failures, envelope failures, malformed directives, and case-file failures stop
manifest loading before skip evaluation, fixture copying, or command startup.
Failures identify the indexed message when the malformed input follows a
complete preceding array element. Failures identify the JSON value position
when a parsed value supplies that context. The `manifest_jsonrpc_*` tests in
`toolchain_harness.rs` are the executable evidence for the accepted envelope
matrix, exact framing, recursive expansion, resource boundaries, and lifecycle
ordering. Existing `stdin`, `stdin_file`, and raw `.raw` framing cases keep
their prior behavior.

Use repeatable `[[lsp_assert]]` sections to check decoded JSON-RPC messages
from `veln lsp` stdout. Each section selects exactly one response with `id`, or
one notification with `method` and an optional zero-based `occurrence` that
defaults to zero. It then selects a value with an RFC 6901 JSON Pointer in
`path`, including the empty pointer for the complete message. Pointer syntax is
validated while the manifest loads.

Each LSP assertion declares exactly one of `equals`, `equals_file`, `contains`,
or `missing = true`. `equals` uses the common JSON equality rules below.
`equals_file` and `contains` require the selected value to be a JSON string. A
missing path can satisfy `missing = true` only after its response or
notification exists.

The harness requires stdout to be a complete ordered sequence of
`Content-Length` frames before it evaluates LSP assertions. Malformed or
partial framing, trailing bytes, invalid JSON message bodies, and duplicate
response identifiers fail decoded assertions for that invocation. Raw stdout
checks still run independently. Repeated invocations decode and assert their
own streams; failures are reported by run and manifest assertion order. The
semantic baseline records each LSP assertion selector, path, operation, and
operand so migrated cases stay reviewable. The
`decoded_lsp_*`, `raw_stdout_and_decoded_lsp_*`, and
`repeated_run_failures_*` tests in `toolchain_harness.rs` cover the transport,
selector, pointer, operation, independence, and aggregation boundaries.

The `publish-diagnostics`, `semantic-tokens`, and
`semantic-tokens-unsaved-change` LSP cases use structured request fixtures and
case-text sidecars. Their decoded assertions cover initialization capability
values, non-empty and cleared diagnostic notifications, complete semantic
token data, and shutdown responses. Raw LSP cases remain only where protocol
framing or an as-yet-unmigrated representation is still part of the fixture.

The `decoded_mcp_jsonl_*` and `manifest_mcp_assertions_*` tests in
`toolchain_harness.rs` cover MCP JSONL decoding, ID selection, pointer
escaping, equality, ordered arrays, length, missing paths, dynamic workspace
URIs, and rejection boundaries. The `definition-workspace` MCP specification
case uses decoded MCP assertions for response IDs 3 through 11 and keeps raw
stdout fragments only for incidental initialization and tool discovery text.

Use `[[json_assert]]`, `[[result_value_assert]]`, and `[[diagnostics]]` for
semantic checks inside JSON stdout. JSON and result-value assertions accept
`equals`, `equals_file`, `equals_json_file`, or `missing = true`.
`equals_file` compares the selected JSON value as a string and never reparses
the sidecar as JSON. `equals_json_file` parses the sidecar as JSON before the
comparison and uses the common JSON equality rules.

The `equals` operation in `[[json_assert]]`, `[[result_value_assert]]`,
`[[lsp_assert]]`, and `[[mcp_assert]]` compares JSON values recursively. Null,
boolean, and string values require the same JSON kind and decoded value. JSON
numbers require the same complete spelling, so `1`, `1.0`, and `1e0` are
distinct. Every affected section accepts those forms as a complete inline
`equals` value. Arrays require the same length and equal values at each index.
Objects require the same member names and recursively equal member values, but
member order does not affect equality. Existing `equals_json_file` operations
use these same rules.
The shared JSON parser stores parsed JSON numbers with their complete source
spelling. Veln-produced integer values remain integer JSON values when command
outputs, diagnostics, repair results, metrics baselines, or parsed
result-value assertions construct them directly. When `[[result_value_assert]]`
parses a rendered result value, integer atoms that fit the harness integer
storage remain integer JSON values, while decimal and exponent JSON number
atoms keep their complete spelling. A parsed JSON integer token compares equal
to a directly constructed integer only when the decimal spelling is identical.
Parsed decimal and exponent tokens remain distinct JSON numbers and do not
become integer-compatible values for non-harness consumers.
`[[result_value_assert]]` reads a rendered result-failure value string from
`value_path`, wraps it as the outer `Err`, and then checks a parsed value path.
Each JSON or result-value assertion must declare exactly one operation.
`missing` is only valid as `missing = true`. The manifest loader rejects an
omitted operation or `missing = false` before it reads later file-backed
operands.

Use `[[file_assert]]` to check command-written files. The command output path
is read from the copied project after execution, while `equals_file` is still
read from the immutable discovered case. Each file assertion must declare
exactly one of `equals` or `equals_file` before later file-backed operands are
read. Diagnostics and help fragments also accept file-backed text operands
where their inline string forms are accepted. Diagnostic exact messages use
`message_file`. Manifest-failure fragment checks use `[manifest_error]`
`contains_file` and `contains_files`.

Use `[help]` for command help output. It checks a help stream, defaulting to
stdout, through stable help fragments instead of full-output equality. Its
fields are `stream`, `summary`, `usage`, `commands`, `arguments`, `options`,
`contains`, `contains_file`, and `contains_files`. `stream` is `"stdout"` or
`"stderr"`. `summary` checks the first help line, `usage` checks the `Usage:`
line, and the list fields check that the matching section heading and listed
entries appear. Help cases should still use `[stdout]` and `[stderr]` for
stream format and emptiness, and should point behavior questions to the
command specification.

File-backed manifest operands use a case-relative portable path. The path
cannot escape the case directory, traverse a link-like entry, or name anything
except a regular UTF-8 file. Link-like operands fail at the offending entry
without following or exposing the target. The harness reads and validates
these files before skip evaluation, fixture setup, or command execution.
Repeated invocations reuse the same discovered text snapshot. Checked-in
files under `case-text/` are checked out as LF-normalized text unless the path
ends in `.raw`, which reserves an exact-byte fixture convention for content
that must not be line-ending normalized.

The `manifest_sidecar_*` tests in `toolchain_harness.rs` are the executable
evidence for sidecar path grammar, link rejection, operand cardinality, UTF-8
validation before skip evaluation, exact text comparison, snapshot timing, and
lifecycle ordering.

Use `[[binary_fixture]]` and `[[output_chunk_list]]` only for test-owned binary
fixture evidence. Binary fixture records compare named program-output lines
against complete lowercase hex, decoded counts, optional consumed counts,
stable fixture errors, and byte diagnostic metadata for truncation or invalid
field checks. Output chunk lists compare a named, ordered sequence of complete
lowercase hex chunks against consecutive program-output lines, including empty
lists and zero-length chunks.

## Manifest Policy

Case manifests are declarative. They should describe the command, expected exit
status, expected stdout or stderr fragments, and structured JSON expectations.
They must not execute arbitrary shell commands.

Use `stdin` only for protocol-style command input that is part of the fixture,
such as LSP exchanges. Use `[requires]` for host capabilities the case needs,
and `[skip]` for platform-specific exclusions with an explicit reason.

Use `[env]` for fixed environment variables that belong to the fixture. Use
`repeat` when one isolated project should run the same command more than once.
Repeated invocations can check stable stdout, stderr, exit status, JSON, file
results, and other command-visible state changes.

Use `[tools]` for controlled external tool availability owned by the harness.
The implemented keys are `java` and `git`. Java accepts `"missing"`,
`"fake-success"`, and `"real"`. Git accepts `"missing"` and `"real"`.
`"missing"` runs the command with an isolated tool path that contains no
launcher for that tool. Java `"fake-success"` installs a harness-owned wrapper
that exits successfully without running arbitrary manifest code. `"real"`
exposes the host launcher under the isolated tool path; Java cases that use it
should also declare `[requires] jdk = true`.

Test harness-owned tool setup with harness or runner unit tests. Do not add CLI
cases solely to prove Java launcher setup, because JVM availability and wrapper
mechanics are not Veln command behavior.

JSON output should be parsed and checked semantically by default. Full JSON
equality is reserved for schema smoke tests where exact envelope shape is the
behavior under test.

## Pre-Migration Semantic Baseline

`toolchain-case-semantics.baseline` is the schema-versioned contract inventory
for every parsed `case.toml` under `crates/veln-cli/tests/toolchain_cases/` and
`examples/specification/`. It records ordered invocation and assertion fields,
typed values, execution gates, case digests, and an aggregate digest. Large
text values, including nested strings inside typed JSON assertions, record an
explicit logical field, byte length, and SHA-256 digest. Binary values record
their byte length and SHA-256 digest. JSON object members are key-sorted
because object member order is not part of an assertion value; arrays and all
manifest assertion sequences retain their order. JSON number tokens retain
their complete spelling, including integer, decimal, exponent, and negative
zero forms. The baseline records file-backed JSON assertion operands under the
operation that supplied them, so `equals_file` and `equals_json_file` remain
reviewable as distinct assertion contracts. The baseline includes MCP stdio
specification cases that use `stdin_file` JSON lines and stream fragments to
pin advertised tool declarations and representative tool results.

The normal `toolchain_harness` target runs
`checked_in_semantic_baseline_matches_authoritative_cases`. The test reads the
baseline and current manifests from the shared discovery inventory without
writing either one. A mismatch reports added or removed cases before reporting
case-qualified field differences.
Run the focused non-mutating check with:

```sh
cargo test -p veln-cli --test toolchain_harness \
  toolchain_semantic_baseline::checked_in_semantic_baseline_matches_authoritative_cases \
  -- --exact
```

Candidate generation is an explicit ignored test. Generate a candidate only
for deliberate contract review. Do not replace the checked-in baseline merely
to accept an unexplained difference.

```sh
VELN_TOOLCHAIN_SOURCE_GIT_TREE="$(git rev-parse HEAD^{tree})" \
VELN_TOOLCHAIN_BASELINE_CANDIDATE=target/toolchain-case-semantics.candidate \
cargo test -p veln-cli --test toolchain_harness \
  toolchain_semantic_baseline::generate_toolchain_semantic_baseline_candidate \
  -- --ignored --exact
```

## Source-Error Guard

Specification examples reject unexpected source diagnostics unless the manifest
sets `source_errors = "expected"` or the command expectation intentionally
checks a source diagnostic. The failure message includes the diagnostic
locations and identifiers needed to clean the example or mark the source error
as intentional.

For normal `check`, `run`, and `test` cases, the harness does not run an
independent whole-project analysis before the CLI invocation. It asks the real
command process to write a harness-owned source diagnostic artifact for the
copied project and run. The artifact is internal test evidence; it does not
change stdout, stderr, exit status, JSON output, generated files, or command
semantics.

The artifact contains checked diagnostics for the copied project root, not only
for the command-selected source inputs. This preserves the source-error guard
for explicit-input `run` and `test` cases that otherwise analyze only a
selected file before execution. Each repeated invocation writes a distinct
artifact path, so one run or copied project cannot satisfy another run's
guard.

Cases with `source_errors = "expected"` keep the independent guard because some
examples intentionally contain source errors outside the command-selected
slice. `doc`, `fmt`, `lsp`, and `repair` also keep the independent guard until
those commands expose equivalent checked evidence to the harness.

## Analysis Cost Evidence

The duplicate source-error analysis removed from normal `check`, `run`, and
`test` cases was the harness-owned `checked_project_diagnostics` call before
the CLI invocation. The real command now produces the checked diagnostic
artifact that the guard reads. Harness boundary tests verify that a clean
copied project does not satisfy a later dirty copied project, and that a
repeated invocation reads the artifact generated for that invocation after the
copied project changes.

Controlled measurements used the same prebuilt debug toolchain for the direct
CLI invocation and the harness case. Each value below is the median of five
measured runs. The previous measurements recorded the small schema direct run
at 0.43 seconds and the HTTP/2 core toolchain case at 13.56 seconds.

After the artifact guard change, representative debug-toolchain observations
were:

| Workload | Direct CLI | Harness case | Ratio |
| --- | ---: | ---: | ---: |
| Binary schema decode step | 0.40 seconds | 0.47 seconds | 1.17 |
| HTTP/2 protocol core closed JSON | 4.22 seconds | 4.32 seconds | 1.02 |

These observations are local review evidence, not CI failure thresholds.

## Toolchain Analysis Benchmark

Use `scripts/benchmark-toolchain-analysis compare BASELINE_BINARY NEW_BINARY`
when reviewing the bounded toolchain-analysis proposal's controlled benchmark.
The command expects prebuilt CLI binaries. It does not build either binary
during measured runs.

The benchmark covers the small schema, HPACK static codec, HTTP/2 core, HTTP/2
connection, and three generated unrelated fully annotated module-graph
workloads at adjacent doubling sizes. It keeps generated projects in temporary
storage and compares measured-run exit status and normalized functional
output before reporting threshold results. When `--output PATH` is supplied,
it writes deterministic JSON with the exact binary path and command used for
every workload.

The optional toolchain-case overhead comparison needs an explicit command
because it is not part of the CLI binary interface. Set
`VELN_TOOLCHAIN_CASE_COMMAND` to include that workload in a comparison run.
Without that environment variable, the benchmark reports the comparison as
skipped.

## Boundaries

The harness standardizes CLI integration tests. It does not replace parser,
checker, runtime, or formatter unit tests in compiler crates.

Use the language specification when a case needs to decide whether command,
diagnostic, JSON, runtime, or source behavior is correct. Use this page only
for harness organization and assertion policy.
