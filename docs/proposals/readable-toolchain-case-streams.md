---
role: proposal
review-when: The toolchain case manifest grammar, stream fixture formats, JSON-RPC harness assertions, migration inventory, or completion gate changes.
---

# Readable Toolchain Case Streams

## Summary

Make multiline toolchain inputs and expected outputs readable without embedding
line-break escape sequences in `case.toml`. The harness will support three
complementary representations:

1. TOML-compatible multiline strings and multiline arrays for short content;
2. case-relative sidecar files for large or independently useful exact text;
3. structured JSON-RPC request fixtures and response assertions for LSP cases.

The migration is complete when every discovered toolchain `case.toml` is free
of encoded line breaks, all migrated cases preserve their previous observable
checks, and the harness rejects later manifest regressions.

## Motivation

The harness currently parses each manifest one line at a time. String values
therefore encode every line break and place large inputs or outputs on one
physical line. This makes source text, command output, JSON-RPC messages, and
long fragment arrays difficult to review.

The current inventory contains 1,504 manifests under the two discovery roots.
Of those manifests, 611 contain an encoded newline on 741 manifest lines. The
affected values consist mainly of exact `equals` strings, LSP `stdin`, and
multiline `contains` fragments. One exact output occupies approximately 39 KiB
on one manifest line.

TOML has multiline basic strings and multiline literal strings, but it has no
heredoc construct. The current harness grammar is also not complete TOML. For
example, assertion values accept JSON `null`. This proposal adds the useful
TOML-compatible forms without requiring an unrelated full manifest-language
migration.

## Goals

- Make short multiline text readable in the manifest where it is asserted.
- Keep large exact content in a plainly reviewable sidecar file.
- Remove manual JSON-RPC framing and `Content-Length` maintenance from LSP
  cases.
- Assert LSP responses as decoded messages instead of escaped stdout
  substrings when message structure is the intended evidence.
- Preserve exact bytes, final line breaks, assertion meaning, and case
  isolation during migration.
- Make the no-encoded-line-break rule mechanically enforceable for all current
  and future toolchain manifests.

## Non-Goals

- Replacing the complete custom manifest grammar with a general TOML parser.
- Requiring every string or every expected output to use a sidecar file.
- Replacing semantic JSON assertions with full-output snapshots.
- Changing Veln command output, LSP behavior, or JSON-RPC framing.
- Banning line-break escapes from Veln source fixtures or from fixtures whose
  subject is the spelling of an escape sequence.
- Reformatting unrelated single-line manifest values.

## Representation Choice

Authors select the smallest readable representation that preserves the
assertion's intent.

| Content | Required representation |
| --- | --- |
| Short text whose line structure is useful beside the assertion | Multiline string in `case.toml` |
| A list with long or multiline elements | Multiline array containing multiline strings where needed |
| Large exact text, reusable text, or content best reviewed in its native form | Case-relative sidecar file |
| JSON-RPC requests sent to `veln lsp` | Structured JSON-RPC request fixture |
| JSON-RPC response fields or notifications | Decoded message assertion |
| A large text value inside a JSON-RPC request or response assertion | Case-relative sidecar file referenced by the structured fixture or assertion |

The harness does not choose a representation from a size threshold. The
manifest author makes that choice, subject to the completion gate.

## Multiline Manifest Values

The manifest grammar will accept TOML-compatible multiline basic strings,
multiline literal strings, and multiline arrays in every field that currently
accepts the corresponding single-line value.

```toml
[[json_assert]]
path = "stdout"
equals = '''
first line
second line
'''
```

```toml
[stdout]
contains = [
  "stable heading",
  '''
  first detail line
  second detail line
  ''',
]
```

The decoded values follow TOML multiline-string rules. In particular, the
newline immediately after the opening delimiter is not part of the value.
Subsequent line breaks are part of the value. A closing delimiter on its own
line therefore leaves the preceding line break in the value.

Multiline literal strings do not process backslash escapes. Multiline basic
strings retain the supported escape behavior for cases that need it. Authors
use literal strings by default when the expected text contains quotes or
backslashes.

The parser reports an unterminated multiline value at its opening line. A
section header, comment marker, or assignment-looking line inside a multiline
string is content, not manifest structure.

## Case-Relative Text Files

The root invocation accepts `stdin_file` as an alternative to `stdin`.
Assertions that accept an exact string value accept `equals_file` as an
alternative to `equals`. This includes JSON string assertions, parsed result
value assertions, and file content assertions. Raw `[stdout]` and `[stderr]`
sections accept `equals_file`; the captured stream must equal the complete file
contents. A raw stream exact-file check may coexist with its format and fragment
checks, and every configured check must pass.

```toml
stdin_file = "request.txt"

[[json_assert]]
path = "stdout"
equals_file = "expected.stdout"
```

The following rules apply to every case file reference:

- The path is relative to the directory that contains `case.toml`.
- The path must not be absolute and must not contain a parent traversal.
- No path component may be a symbolic link.
- The target must be a regular file inside the copied case fixture.
- The harness reads the file as UTF-8 and preserves its complete contents,
  including a final line break.
- A missing, unreadable, non-UTF-8, or escaping file fails manifest loading
  with the manifest field and relative path.
- An inline value and its file alternative are mutually exclusive.
- A field that requires one exact value fails when both alternatives are
  absent.

Sidecar files are fixture inputs. The harness copies them with the rest of the
case before command execution. An assertion reads its expected sidecar from
the immutable discovered case, not from a path that the command can overwrite.

## Structured JSON-RPC Fixtures

An LSP case may use `stdin_jsonrpc_file` instead of `stdin` or `stdin_file`.
The referenced UTF-8 JSON file contains an ordered array of JSON-RPC message
objects. The harness serializes each object, calculates its UTF-8 byte length,
and writes the standard `Content-Length` frame. Message order is the array
order.

```toml
command = ["lsp"]
stdin_jsonrpc_file = "requests.json"
```

The request fixture may replace a complete JSON value with a case text file:

```json
{
  "$case_text": "opened.veln"
}
```

The directive object must contain only `$case_text`. The harness replaces it
with the exact UTF-8 contents of the named case-relative file before JSON
serialization. This permits open-document and change-document text to remain
in a native source file without JSON line-break escapes. The case file path
uses the same containment and immutability rules as `equals_file`.

The harness rejects malformed JSON, a non-array root, a non-object message,
an invalid case-text directive, and a message that cannot be framed. It does
not start the command after one of these fixture errors.

### Decoded Response Assertions

The harness decodes framed stdout from an LSP invocation into an ordered
message sequence. An `[[lsp_assert]]` selects either a response by `id` or a
notification by `method` and occurrence. It then applies a JSON path assertion
to the decoded message.

```toml
[[lsp_assert]]
id = 2
path = "result.uri"
contains = "/main.veln"

[[lsp_assert]]
method = "textDocument/publishDiagnostics"
occurrence = 0
path = "params.diagnostics.0.code"
equals = "name.unresolved"
```

Exactly one of `id` and `method` is required. `occurrence` is zero-based and is
valid only with `method`; it defaults to zero. The selected message must be
unique after applying the selector and occurrence. The assertion supports the
`equals`, `equals_file`, `missing`, and string `contains` operations. Exactly
one operation is required. `equals_file` compares a JSON string with the exact
file contents. `contains` requires a JSON string containing the configured
substring. Missing messages, duplicate response identifiers, invalid paths,
wrong JSON value kinds, and comparison failures identify the selector and
assertion path.

Raw stdout assertions remain available for framing-specific cases. Ordinary
LSP behavior cases use decoded response assertions so JSON serialization is
not part of their expected behavior.

## Compatibility And Migration

Existing single-line values remain accepted while migration is in progress.
The new representations must produce the same invocation bytes and the same
expected values as their old inline forms.

Each migrated case retains its current semantic assertion boundary:

- exact output remains exact output;
- fragment checks remain fragment checks unless a decoded LSP assertion is a
  more direct expression of the same behavior;
- JSON values remain semantically parsed values;
- final line-break presence remains significant; and
- repeated invocations continue to reuse the same immutable expected fixture.

Migration must not replace stable semantic assertions with broad snapshots
only to move text out of a manifest.

The migration inventory covers both harness discovery roots:

- `crates/veln-cli/tests/toolchain_cases/`;
- `examples/specification/`.

No allowlist is provided for encoded line breaks in `case.toml`. A case that
intentionally checks the two-character spelling of an escape sequence stores
that expected content in a sidecar file instead.

## Acceptance Model

All rows describe planned evidence. They do not imply that the behavior is
already implemented.

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Parse multiline basic and literal strings containing quotes, backslashes, assignment-looking lines, comments, and section-looking lines. | Each field receives the specified text and later manifest structure is parsed normally. | Manifest parser unit cases. |
| Parse multiline arrays containing single-line and multiline strings with trailing commas and comments. | Array order and decoded element values are preserved. | Manifest parser unit cases. |
| Leave a multiline value unterminated. | Manifest loading fails at the opening line and does not run the command. | Manifest parser failure case. |
| Set both an inline exact value and its file alternative. | Manifest loading rejects the ambiguous field. | Table-driven manifest validation cases. |
| Reference a valid sidecar with and without a final line break. | The command or assertion receives the exact file contents. | Invocation and assertion unit cases. |
| Reference an absolute, parent-traversing, missing, non-regular, or non-UTF-8 sidecar. | Manifest loading rejects the field before command execution. | Sidecar boundary matrix. |
| Let a command overwrite a copied file with the same relative name as an expected sidecar. | The expected value remains the discovered fixture value. | Immutable expectation boundary case. |
| Supply an ordered JSON-RPC request array. | The child receives one correctly sized frame per array element in array order. | Framing unit case with ASCII, multiline, and non-ASCII values. |
| Use `$case_text` for open-document text. | The framed JSON string equals the exact sidecar text and its byte length is correct. | JSON-RPC substitution and framing case. |
| Supply malformed request JSON, an invalid root or message, or an invalid `$case_text` directive. | Fixture loading fails before the LSP command starts. | JSON-RPC fixture failure matrix. |
| Assert response and notification values by selector and path. | The harness decodes frames and reports comparison results independent of compact JSON spelling. | LSP assertion unit cases. |
| Select a missing message, duplicate response identifier, out-of-range notification occurrence, or invalid path. | The failure identifies the selector and assertion path. | LSP assertion failure matrix. |
| Run representative migrated exact-output, fragment, formatting, repeated-run, and LSP cases. | Their observable checks match the pre-migration cases. | Focused migrated toolchain cases. |
| Discover any `case.toml` containing an encoded LF or CR spelling. | Harness validation fails before running that case and identifies the manifest and spelling. | Repository-wide manifest policy test with a synthetic failing fixture. |
| Discover all checked-in toolchain manifests after migration. | No manifest policy violation is reported and the complete harness suite passes. | Existing toolchain harness test target. |

## Verification Route

The planned parser, fixture, assertion, migration-policy, and discovered-case
evidence runs through:

```sh
cargo test -p veln-cli --test toolchain_harness
```

The implementation may provide narrower test filters for development, but the
unfiltered target is the completion evidence.

## Completion Gate

The proposal is complete only when all of the following conditions hold:

1. The multiline grammar, sidecar fields, structured JSON-RPC input, decoded
   LSP assertions, and their failure behavior pass the planned evidence above.
2. Every discovered toolchain case has been migrated without weakening its
   assertion boundary.
3. No `case.toml` in either discovery root contains `\\n`, `\\r`, or an
   equivalent numeric encoding of LF or CR used to hide an embedded line
   break.
4. A harness-owned manifest policy test scans the same discovered manifest set
   and fails on a later violation. The check runs through the existing
   toolchain harness test target; it does not require a separate CI workflow or
   an `AGENTS.md` rule.
5. The complete toolchain harness suite passes after migration.
6. The implemented harness contract is documented in
   `../reference/toolchain-test-harness.md`.
7. The completed proposal is removed from `docs/proposals/` and its durable
   rationale or completion evidence is retained under
   `../reference/implemented-proposals/` only if it remains useful.

The zero-escape scan is necessary but not sufficient. It proves the textual
migration target, while the parser, sidecar, JSON-RPC, semantic-equivalence,
and full-suite evidence prove that moving the content did not weaken or change
the tests.

## Implementation Slices

1. Add multiline string and multiline array parser coverage without migrating
   cases.
2. Add case-relative text operands, containment checks, immutable expectation
   reads, and boundary coverage.
3. Add structured JSON-RPC request framing, `$case_text`, decoded LSP
   assertions, and protocol fixture coverage.
4. Migrate a representative case from each assertion shape and confirm
   semantic equivalence.
5. Migrate both discovery roots and add the repository-wide manifest policy
   test.
6. Run the complete harness suite, update the harness reference, and complete
   the proposal lifecycle audit.

## Guardrail Placement

The regression risk is a deterministic property of harness-owned manifests.
The narrowest effective guardrail is therefore a harness test that uses the
same discovery logic as case execution. A prose-only convention would not
protect human and automated contributors equally. A new CI job would duplicate
the existing test route, and an always-on repository instruction would expose
a harness-specific rule to unrelated work.
