---
role: proposal
update-when: The toolchain case semantic baseline, manifest grammar, stream fixture formats, JSON-RPC harness assertions, migration inventory, implementation status, or completion gate changes.
---

# Readable Toolchain Case Streams

## Summary

Make multiline toolchain inputs and expected outputs readable without embedding
line-break escape sequences in `case.toml`. The implemented manifest syntax
supports TOML-compatible multiline strings and multiline arrays for short
content. The remaining slices add two complementary representations and the
migration policy around all three forms:

1. case-relative sidecar files for large or independently useful exact text;
2. structured JSON-RPC request fixtures and response assertions for LSP cases.

The migration is complete when every discovered toolchain `case.toml` is free
of encoded line breaks, all migrated cases preserve their previous observable
checks, and the harness rejects later manifest regressions.

## Implementation Status

The pre-migration semantic baseline slice is implemented. The checked-in
baseline describes every case under both authoritative roots through the
current parsed manifest model. The normal toolchain harness target compares
that baseline with the current inventory without changing either artifact.

The manifest-syntax foundation is also implemented. The harness has one
physical-line-aware lexer/parser boundary and accepts the resolved TOML string,
string-array, multiline JSON value, physical-newline, and error-location
contracts. The current contract and executable evidence are documented in
[Toolchain Test Harness](../reference/toolchain-test-harness.md#manifest-value-syntax).

Canonical discovery, preflight policy, sidecar operands, manifest migrations,
conversion records, structured JSON-RPC support, and the encoded-line-break
policy remain planned. The implementation slices below retain their dependency
order; the implemented syntax subset does not imply that the rest of the first
slice is complete.

## Review Route

- Start with [Representation Choice](#representation-choice) for the authoring
  surface and [Compatibility And Migration](#compatibility-and-migration) for
  the preservation boundary.
- Use [Resolved Design Decisions](#resolved-design-decisions) when reviewing
  filesystem, policy, JSON, or LSP semantics. Implemented manifest value syntax
  belongs in
  [Toolchain Test Harness](../reference/toolchain-test-harness.md#manifest-value-syntax).
- Use [Acceptance Model](#acceptance-model) as the implementation checklist.
- Use [Completion Gate](#completion-gate) and
  [Implementation Slices](#implementation-slices) for rollout and closure.

## Motivation

The harness previously parsed each manifest one line at a time. Existing case
values still encode every line break and place large inputs or outputs on one
physical line because case migration remains planned. This makes source text,
command output, JSON-RPC messages, and long fragment arrays difficult to
review.

The current inventory contains 1,508 manifests under the two discovery roots.
Of those manifests, 611 contain an encoded newline on 741 manifest lines. The
affected values consist mainly of exact `equals` strings, LSP `stdin`, and
multiline `contains` fragments. One exact output occupies approximately 39 KiB
on one manifest line.

TOML has multiline basic strings and multiline literal strings, but it has no
heredoc construct. The current harness grammar is also not complete TOML. For
example, assertion values accept JSON `null`. This proposal adds the useful
TOML-compatible forms without requiring an unrelated full manifest-language
migration.

## Remaining Goals

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
| Invalid or presentation-sensitive JSON-RPC framing | Exact raw sidecar file |
| A large text value inside a JSON-RPC request or response assertion | Case-relative sidecar file referenced by the structured fixture or assertion |

The harness does not choose a representation from a size threshold. The
manifest author makes that choice, subject to the completion gate.

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

Every case file reference follows the portable grammar and no-follow boundary
defined below. In summary:

- The path is relative to the directory that contains `case.toml`.
- The decoded path uses portable ASCII components and `/` separators only.
- No traversed component may be a link or Windows reparse point.
- The discovered target and its copied counterpart must be regular files.
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
path = "/result/uri"
contains = "/main.veln"

[[lsp_assert]]
method = "textDocument/publishDiagnostics"
occurrence = 0
path = "/params/diagnostics/0/code"
equals = "name.unresolved"
```

Exactly one of `id` and `method` is required. `occurrence` is zero-based and is
valid only with `method`; it defaults to zero. Response and notification
cardinality follows the resolved rules below. The assertion supports the
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
Multiline and exact-file representations must produce the same invocation
bytes and expected values as their old inline forms. A reviewed raw-to-
structured JSON-RPC conversion instead preserves the ordered decoded message
values and uses the deterministic framing contract below.

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

## Resolved Design Decisions

### Implemented Manifest Syntax

The TOML string, string-array, multiline JSON value, physical-newline, and
error-location contracts are implemented, including malformed and incomplete
Unicode escape failures. The current normative route and executable evidence are
[Toolchain Test Harness](../reference/toolchain-test-harness.md#manifest-value-syntax).

### Portable Case-File References

A case-file reference is a non-empty sequence of portable components separated
only by `/`. A component contains one or more ASCII letters, ASCII digits,
periods, hyphens, or underscores. A component must not be `.` or `..` and must
not end in a period. Its stem before the first period must not equal `CON`,
`PRN`, `AUX`, `NUL`, `COM1` through `COM9`, or `LPT1` through `LPT9` under
ASCII case-insensitive comparison.

Leading, trailing, and repeated separators are invalid. Backslashes, colons,
spaces, controls, non-ASCII characters, and all other punctuation are invalid.
Validation applies after manifest string decoding, so an escape cannot bypass
the grammar. The harness does not normalize, percent-decode, case-fold, expand,
or remove components.

The validator produces a component sequence before invoking any host path API.
Resolution walks that sequence relative to the case directory and requires the
stored spelling of every directory entry to match exactly. The same sequence
addresses the discovered and copied fixtures. Link, containment, lifetime, and
file-content checks occur in later validation stages.

### Fixture Containment Threat Model

The harness treats each discovered fixture as a trusted, quiescent tree while
it validates references, snapshots referenced files, and copies the case. It
rejects a symbolic link or Windows reparse point in any reference component,
including links whose targets remain inside the fixture. The final target must
be a regular file. The copy walk also rejects every link, reparse point, and
non-directory or non-regular entry, including entries that no manifest field
references. The copied fixture is link-free when command execution begins.

Referenced input and expected bytes come from the validated discovered file
and are not resolved again after command execution begins. The copied tree is
mutable execution input, so a command may later replace its entries without
changing those bytes.

The containment guarantee does not survive adversarial concurrent replacement
between filesystem operations. Hard links are treated as regular files. The
harness does not claim protection from writes through another hard link,
mount replacement, network filesystem behavior, or links above a configured
discovery root. These boundaries are explicit because the harness is not a
filesystem security sandbox.

### Case-File Snapshot Lifecycle

After manifest and operand validation, the harness resolves every referenced
file against the discovered fixture before it evaluates skip rules. It reads
each distinct portable path once. All operands that name the same path share
one immutable per-case snapshot. Structured fixtures are parsed and all
`$case_text` directives are resolved from those snapshots during the same load
phase.

Loading is transactional. References are processed in manifest source order,
and the first failure identifies its field and path. If any reference fails,
the harness discards the partial resolved case and does not copy a project,
configure tools, or start a command. A skipped case must therefore retain
complete, valid referenced fixtures. A valid skipped case is not copied or
executed after resource validation succeeds.

Every repeated invocation and assertion reuses the resolved snapshots. The
harness does not reread either the discovered or copied path after execution
begins. This rule does not reset other copied-project state between repeats and
does not extend the concurrent-mutation guarantee beyond the containment
threat model above.

### Text Sidecar Bytes

A plain text sidecar contains valid UTF-8 and preserves every byte. A leading
UTF-8 byte-order mark is the first U+FEFF value character. CRLF, LF, a lone CR,
and the presence or absence of a final line break are content. The harness does
not strip, normalize, or synthesize any of them. `stdin_file` writes those
bytes, `equals_file` compares them, and `$case_text` inserts the same decoded
characters into a JSON string.

A structured JSON request fixture is a JSON document rather than an exact text
operand. It rejects a leading UTF-8 byte-order mark. LF and CRLF between JSON
tokens are whitespace and do not affect decoded messages or serialized frame
bodies. A raw newline inside a JSON string is invalid. Text inserted through
`$case_text` remains data, so its U+FEFF, CR, LF, and final line break remain in
the selected message value. Frame length is calculated from the serialized
UTF-8 body after substitution.

Repository attributes make checked-in bytes independent of checkout settings.
Ordinary files under both discovery roots use `text eol=lf`. A fixture whose
subject requires repository-recorded CR bytes uses the designated `.raw`
suffix, which uses `-text`. The harness still preserves the bytes of an
untracked or synthetic valid UTF-8 sidecar, but the repository portability
guarantee applies only to checked-in fixtures governed by those attributes.

### Operand Cardinality

Cardinality counts field presence, not the operand's value. An empty inline
string, an empty sidecar, and `equals = null` are present operands. Explicit
`missing = false` is invalid rather than an omitted operation. Repeating an
operand or choice key in one scope is invalid. Cardinality is validated before
any referenced file is read.

| Scope | Alternatives | Required cardinality and type |
| --- | --- | --- |
| Root invocation | `stdin`, `stdin_file`, `stdin_jsonrpc_file` | Zero or one. Structured input is valid only when the command's first argument is `lsp`. |
| `[stdout]` or `[stderr]` | `equals_file` | Zero or one. It may coexist with `format`, `contains`, and `not_contains`; every configured check must pass. Raw inline `equals` and `missing` are unsupported. |
| `[[json_assert]]` | `equals`, `equals_file`, `missing = true` | Exactly one. Inline equality accepts any JSON value. File equality expects a JSON string and does not parse the sidecar as JSON. |
| `[[result_value_assert]]` | `equals`, `equals_file`, `missing = true` | Exactly one, with the same value-kind rules as `json_assert`. |
| `[[file_assert]]` | `equals`, `equals_file` | Exactly one exact text expectation. The actual `path` remains separately required. |
| `[[lsp_assert]]` selector | `id`, `method` | Exactly one. `occurrence` is valid only with `method`, and `path` remains required. |
| `[[lsp_assert]]` operation | `equals`, `equals_file`, `contains`, `missing = true` | Exactly one. Inline equality accepts any JSON value. File equality and `contains` require the selected value to be a string. |

For JSON, result-value, and LSP assertions, `missing = true` tests absence of
the assertion path after its enclosing value or message has been selected. A
missing LSP message is a selector failure, not a successful missing-path
assertion. For a file assertion, the actual `path` is read from the copied
project after execution while `equals_file` remains a discovered snapshot.

### Authoritative Case Discovery

One no-follow structural inventory is authoritative for case-test generation,
migration inventory, and manifest policy validation. It scans the two
configured discovery roots, records each regular `case.toml` by root identity
and slash-separated relative case directory, and sorts those descriptors by
their UTF-8 spelling. Generated execution tests and the policy test consume the
same descriptor list instead of implementing separate filesystem walks.

A configured discovery root is a container and must not itself contain
`case.toml`. A directory with a manifest is a terminal case root for execution,
but structural inventory continues below it far enough to reject any nested
`case.toml`. The failure names both manifests and directs the author to remove
the nested manifest or move it to a sibling case directory, because hidden
manifests would escape execution and policy checks.

Discovery never follows a symbolic link or Windows reparse point. Encountering
one in either root is an error, whether its target is inside or outside the
root. The failure names the entry and directs the author to replace it with a
regular fixture entry, because following or ignoring links could hide cases,
escape a root, or create cycles. Configured roots must not overlap. Skip and
tool requirements never remove a descriptor from migration or policy
validation; they affect command execution only.

### Encoded-Line-Break Policy

The manifest lexer retains source provenance for decoded string characters.
The policy examines every manifest string token and every JSON string token,
including nested JSON object keys and values. It does not use a raw whole-file
scan or a decoded-value-only scan.

A token violates the policy when an escape produces LF or CR. This includes
TOML and JSON `\n`, `\r`, `\u000A`, and `\u000D`, plus TOML
`\U0000000A` and `\U0000000D`, with case-insensitive hexadecimal digits. A
token also violates the policy when its decoded text contains the literal
spelling `\n`, `\r`, `\u000A`, `\u000D`, `\U0000000A`, or
`\U0000000D`. The decoded check catches escaped backslashes, literal strings,
arbitrary backslash-run parity, and spellings assembled from Unicode escapes.

Physical LF or CRLF in a multiline string is allowed. Multiline-basic folding
is allowed when it leaves no forbidden decoded spelling. Comments, sidecars,
structured JSON fixture files, numeric values, and escape-like application
encodings are outside this predicate. Separate string tokens are not
concatenated for policy purposes.

A finding identifies the manifest field, source line and span, forbidden
spelling, and category. It directs the author to use physical multiline text or
a sidecar, and explains that the replacement keeps case manifests reviewable.
Intentional tests of an escape spelling use a sidecar rather than an allowlist.

### Policy Timing And Reporting

The shared discovery, lexer, and encoded-line-break predicate run as a global
build preflight. The preflight scans every authoritative manifest, including
skipped cases, before generating the harness test binary. It aggregates all
reliably tokenized policy findings and all discovery or lexical scan errors.
One malformed manifest does not stop scanning later descriptors. If token
boundaries become unreliable within a manifest, the report does not invent
findings beyond that boundary.

The report sorts findings by root-qualified path, source line, source span, and
category. It reports the total findings and affected manifests, then gives the
repairable fact and location for each finding. The summary directs maintainers
to move encoded text to physical multiline values or sidecars and explains
that escaped line structure hides fixture changes. A failed preflight prevents
test-binary generation, skip evaluation, fixture setup, and case commands.

Each generated case also passes a process-wide one-time runtime barrier before
case loading. The barrier rediscovers the current tree, compares it with the
generated inventory, and reruns the shared scan. Concurrent case threads wait
for the result. This catches a stale prebuilt binary or a tree changed after
build. A parity failure lists added, removed, or relocated manifests and directs
the maintainer to rebuild because policy and execution must use the same set.
Synthetic cases outside generated discovery use the same local predicate before
resource loading. The guarantee covers commands spawned by this harness after
the barrier; it does not cover other processes or adversarial mutation after a
successful preflight.

### Migration Equivalence Evidence

Before manifest migration begins, the unchanged harness emits a checked-in,
schema-versioned semantic contract baseline from the authoritative case
inventory. The baseline records its source Git tree identifier, generator
schema, roots, case count, and aggregate digest. It is generated and reviewed
in a separate implementation slice and is never regenerated from migrated
manifests to make a comparison pass.

Each case descriptor removes representation spelling but preserves observable
contract shape. It includes invocation arguments, working directory, ordered
environment, exact resolved stdin bytes, repeat and execution gates; every
existing assertion kind, selector, path, operation, semantically ordered
sequence, and typed value;
and exact BOM, CRLF, indentation, and final-line-break bytes. Large text uses a
logical field identifier, byte length, and SHA-256 digest rather than another
escaped text copy, including nested large strings inside typed JSON assertion
values. Operation and selector metadata remain visible so a review does not
rely on an aggregate hash alone.

The comparator requires exact case-set and descriptor equality by default.
Equivalent inline, multiline, and file-backed spellings therefore compare
equal, while exact-to-fragment changes, removed or shortened fragments,
operation changes, ordering changes in ordered sequences, path or selector
changes, sidecar byte changes, and unrelated case additions or removals fail.
Object-member reordering alone compares equal because object order is not part
of the JSON value contract.

Raw JSON-RPC input or raw stdout evidence may change to semantic structured
evidence only through a typed conversion record. Each record identifies one
case, exact old and new component fingerprints, every removed and added logical
assertion, the replacement selector and path, the preserved claim, and why JSON
serialization or framing is not under test. Framing-specific cases cannot use
this conversion. Wildcards, directory approvals, generic changed-field
allowances, stale records, unused records, and records that also consume an
unrelated difference are invalid.

The baseline, comparator, and conversion records remain active until every
case is identical or covered by one fully consumed conversion, the encoded-line
break gate is clean, and the complete harness suite passes. Migration-only
artifacts may then be removed so future case changes are not frozen to the old
suite. A compact implemented-proposal record retains the source tree identifier,
aggregate evidence, conversion case identifiers, and verification route only
when that history remains useful.

### Structured Request Envelope

After `$case_text` substitution and before framing, every fixture element must
match the harness's JSON-RPC client request or notification profile. It must be
an object with `jsonrpc` equal to the string `"2.0"` and a string `method`.
An optional `id` is a string, number, or null. An optional `params` is an
object, array, or null; null is an explicit LSP-fixture compatibility extension
to the JSON-RPC structured-value rule. `result` and `error` members are invalid
in this client-input surface. Other members are preserved.

The harness does not validate method names, method-specific parameter schemas,
identifier uniqueness, message order, or LSP lifecycle. Unknown methods and
well-formed envelopes with method-specific invalid parameters still reach the
server under test. All elements validate before any frame is materialized or a
command starts.

A case uses raw `stdin_file` when it tests malformed JSON, a non-object body, a
JSON-RPC batch in one body, an invalid request envelope, a client response, or
invalid framing. Structured input always produces a syntactically valid JSON
object body and a harness-defined valid frame. A fixture failure identifies the
fixture, zero-based element index, member, failed fact, and whether to repair
the envelope or use raw input for an intentional invalidity.

### JSON Interoperability Profile

Structured fixtures and decoded messages use the same profile based on
[RFC 8259](https://www.rfc-editor.org/rfc/rfc8259.html). It accepts the complete
JSON value and number grammar. It intentionally rejects decoded duplicate
object names and unpaired UTF-16 surrogate escapes because those inputs do not
have interoperable value semantics.

A number retains its valid source lexeme and an exact arbitrary-precision
decimal value. The parser accepts integers, fractions, and signed or unsigned
exponents without an `i64` or binary floating-point range. Numeric equality is
mathematical decimal equality, so exponent and trailing-zero spellings of the
same value compare equal and every signed zero equals zero. Serialization emits
the original lexeme and never rounds, overflows, or rewrites it.

Strings accept every standard JSON escape. A high-surrogate `\uXXXX` escape
must be followed immediately by a low-surrogate escape; the pair decodes to one
supplementary Unicode scalar. An isolated, reversed, or malformed surrogate is
invalid. Direct UTF-8 and escaped forms of the same scalar compare equal. No
Unicode normalization or case folding occurs.

Object member names must be unique after string decoding at every depth.
Object equality ignores member order and compares the unique name-value map
recursively. Array equality preserves order. The parsed object also retains
authored member order for structured request serialization, and the serializer
emits that order without sorting. Insignificant whitespace and string escape
choice are not preserved.

### Case-Text Directives

The harness recognizes directive objects at every complete JSON value position
below the fixture's outer array. Recognition uses decoded member names. An
object containing `$case_text` is valid only when that is its sole member and
its value is a portable case-file path string. The complete object becomes one
JSON string containing the immutable text snapshot.

An object containing `$case_literal` is valid only when that is its sole
member. The complete object becomes the member value, and the harness does not
process directives anywhere inside that value. This literal barrier represents
application objects that contain reserved directive member names. An object
that contains a reserved member in any other shape is invalid rather than an
ordinary application object.

Transformation is a single deterministic pass in message, authored object,
and array order. A replacement node is not visited again. Inserted text is not
parsed as JSON, interpreted as a path, or scanned for another directive.
Duplicate decoded member names fail under the JSON profile before directive
processing. After all substitutions and literal unwrapping, every outer-array
element is validated again against the structured request envelope. The entire
fixture succeeds before any frame is emitted or command starts.

### Structured Request Equivalence And Encoding

Migration of an ordinary LSP behavior case to `stdin_jsonrpc_file` preserves
the ordered JSON-profile message values, not the old header or body bytes. Message count
and order, decoded member values, string contents, and array order must match.
Object order, insignificant JSON whitespace, equivalent string escapes,
mathematically equal number spellings, and incidental valid header presentation
may differ. This is the only exception to the general same-invocation-bytes
migration rule, and it requires the typed conversion record defined above.

A transport- or spelling-sensitive case remains raw. This includes malformed,
partial, conflicting, or noncanonical framing under test; malformed or
non-interoperable JSON; body spelling or order sensitivity; and any assertion
whose claim concerns the wire representation. A raw stream that cannot decode
uniquely into complete JSON-profile messages is ineligible for semantic conversion.

The new structured encoding is deterministic even though it need not equal the
old bytes. It emits compact UTF-8 JSON with no byte-order mark or trailing
newline. Arrays and object members use retained authored order. Numbers use
their retained source lexeme. Strings use short escapes for quote, backslash,
backspace, tab, LF, form feed, and CR; other controls use lowercase `\u00xx`;
solidus and non-control Unicode scalars are unescaped.

Each message is framed as ASCII `Content-Length: N\r\n\r\n` followed by the
body, where `N` is the minimal unsigned decimal length of the serialized UTF-8
body. No bytes separate adjacent frames. The harness materializes the complete
sequence before starting the command. Every repeat uses the same bytes.

Migration evidence decodes the baseline raw stream, compares it with the
resolved structured messages, records every discarded presentation dimension,
and replays both inputs against the same LSP binary in independent copied
projects. A semantic mismatch, behavioral replay difference, framing-sensitive
classification, stale fingerprint, or unrelated contract change rejects the
conversion.

### LSP Identifier Domain

An `lsp_assert.id` selector follows the
[LSP base protocol](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
response domain. It is a decoded string, null, or a JSON-profile number whose exact
decimal value is integral and lies in the signed 32-bit LSP integer range.
Boolean, array, object, non-integral, and out-of-range selectors are manifest
errors.

Selection compares JSON kind and decoded value. Strings compare exact Unicode
scalar sequences without normalization. Integers use exact decimal equality,
so integral fraction or exponent spellings of the same value compare equal and
all signed-zero spellings identify zero. A string never equals a number or
null. Null matches only an explicitly present null `id`; it does not match an
absent member and is unrelated to a `missing = true` path operation.

A response-shaped message with a boolean, compound, non-integral, or
out-of-range `id` is invalid LSP output. Decoding and classification fail at
that message instead of silently excluding it from selection. Numeric-equivalent
IDs form one identifier group for the cardinality rules below.

### Decoded LSP Message Classification

When a case contains `lsp_assert`, the harness classifies the complete decoded
message sequence before evaluating any selector. Every body must be a JSON object
whose top-level `jsonrpc` is the string `"2.0"`. Classification uses member
presence, so a present null is never confused with an absent member.

| `method` | `id` | `result` / `error` | Class |
| --- | --- | --- | --- |
| Absent | Valid response ID, including null | Exactly one present | Response |
| String | Absent | Both absent | Notification |
| String | Non-null string or LSP integer | Both absent | Unsupported server request |
| Any other combination | Any | Any | Invalid message |

A response must not contain `params`. A success `result` may be any JSON value.
An `error` must be an object with an in-range LSP integer `code`, a string
`message`, optional arbitrary `data`, and optional non-reserved extensions. A
notification or server request may omit `params` or use an object or array;
scalar and null parameters are invalid. Non-reserved top-level extensions do
not change classification.

An ID selector considers responses only. A method selector considers
notifications only. A server request participates in neither set. Because the
harness cannot conduct an interactive server-request exchange, any server
request fails decoded assertion processing before selector evaluation and
identifies its message index, method, and ID. It is never silently ignored or
misclassified as a notification. A raw-output-only case does not enable this
classification and may retain byte-level evidence for intentional server
request or invalid-message output.

### LSP Selector Cardinality

After complete message classification and before any assertion comparison, the
harness groups every non-null response by the identifier equality above. A repeated
non-null identifier is a global decoded-stream failure even when no assertion
selects it. Success and error responses share the same uniqueness rule, and
numeric-equivalent lexemes collide. Numeric and string identifiers remain
different.

Null does not identify a correlatable request and is not globally unique.
An `id = null` selector nevertheless requires exactly one null-ID response:
zero is missing and more than one is ambiguous. Every non-null ID selector
resolves zero or one response after the global uniqueness check.

A method selector filters classified notifications by exact decoded method and
uses complete frame order. `occurrence` is a zero-based nonnegative integer and
defaults to zero. It selects only the indexed same-method notification;
responses and other methods do not increment it. Additional matches are
allowed. The current surface does not assert a total notification count.

Selection is non-consuming. Any number of assertions may reuse one response or
one `(method, occurrence)` selection to inspect different paths. Selector
resolution and cardinality precede path traversal and comparison. Repeated
invocations build independent candidate groups and never combine messages
across runs.

### Semantic LSP Frame Decoder

Semantic assertions consume stdout as bytes using a strict LSP output profile,
not the incidental tolerance of the server's input reader. A frame has one or
more ASCII header lines terminated by CRLF, then one empty CRLF line, then the
declared body bytes. A header line uses an ASCII field name, exact `: `, and a
nonempty value. Field names compare ASCII case-insensitively and may appear in
either order. LF-only, bare CR, folded lines, whitespace before the colon, and
other separator spellings are invalid.

Exactly one `Content-Length` is required. Its value contains one or more ASCII
decimal digits, may have leading zeroes, and has no sign or whitespace. Parsing
uses checked arithmetic and the value counts UTF-8 body bytes. Every duplicate
is invalid, including an equal duplicate. `Content-Type` is optional and may
appear once. When present, it must name `application/vscode-jsonrpc` with
charset `utf-8` or legacy `utf8`, compared ASCII case-insensitively with ordinary
media-type whitespace. Unknown headers, parameters, media types, and charsets
are invalid.

The decoder consumes exactly the declared body bytes without scanning forward
to resynchronize. The body is strict UTF-8 without a byte-order mark and
contains exactly one JSON-profile object; JSON layout whitespace inside the declared
body is allowed. Envelope classification follows this transport and JSON
validation. Canonical structured-writer output is accepted, while framing-sensitive
or intentionally malformed output remains a raw assertion concern.

### Process Results And Partial LSP Output

Process termination is a capture boundary, not an assertion short circuit.
For each repeated invocation, the harness captures status, stdout, and stderr
completely. It then evaluates the exit check, every raw stdout check, and every
raw stderr check independently. An expected or unexpected nonzero exit, a
signal termination, or one raw-check failure does not suppress another check.
Raw and semantic stdout checks may coexist, and all configured checks must
pass.

When at least one `lsp_assert` is configured, semantic evaluation additionally
requires stdout to be consumed as zero or more complete frames followed
immediately by end of stream. A partial header or body and any trailing byte,
including whitespace outside a frame, make semantic preflight fail. The
harness never evaluates assertions against a valid frame prefix. Empty stdout
is a complete zero-frame sequence, so its selectors fail as missing rather
than as truncated transport. A raw-only case does not invoke the frame decoder
and may inspect intentionally partial or malformed bytes.

One transport, JSON, classification, unsupported-request, or global
cardinality failure blocks every dependent LSP comparison for that invocation.
The harness reports that root failure once and identifies the blocked assertion
locations; it does not turn them into cascading missing-selector failures. If
semantic preflight succeeds, every `lsp_assert` is evaluated independently in
manifest order. One selector, path, or comparison failure does not suppress a
later assertion.

Ordinary assertion failures do not stop later repeats. Each repeat has fresh
frame, identifier, occurrence, selector, and failure state, while retaining
the shared immutable fixture snapshots defined above. Only a harness
infrastructure failure that prevents reliable process launch or capture may
stop the remaining repeats, and it reports the unattempted run count rather
than inventing assertion failures.

The final case report orders findings by repeat number, then exit, raw stdout,
raw stderr, semantic preflight, and manifest assertion order. Large captured
streams appear at most once per repeat. Each finding leads with its specific
failed fact and includes only the relevant expected value, actual value,
stream operation, frame offset, selector, or path. This ordering is independent
of test-thread scheduling.

### LSP Assertion Paths

`lsp_assert.path` uses the string representation of
[RFC 6901 JSON Pointer](https://www.rfc-editor.org/rfc/rfc6901.html). The empty
string selects the complete message. Every nonempty path starts with `/` and
contains slash-prefixed reference tokens. Within one token, `~1` denotes `/`
and `~0` denotes `~`; every other tilde escape is invalid. Decoding performs
the RFC substitutions once in the specified order, so `~01` denotes the key
`~1`, not `/`. URI-fragment and percent-decoded forms are unsupported.

An object token selects the member with the exact decoded Unicode scalar
sequence, without normalization. This permits `/a.b` to select the key `a.b`,
`/0` to select the object key `0`, `/` to select an empty key, and `/a~1b` to
select `a/b`. Duplicate decoded object names have already been rejected by the
JSON profile.

An array token is a canonical zero-based ASCII decimal index: `0` or a
nonzero digit followed by digits. Leading zeroes, signs, whitespace,
non-ASCII digits, and `-` are invalid array traversal rather than aliases or
absence. A canonical index of any magnitude is compared with the array length
without host-width overflow; an index at or beyond that length is absent. A
token applied to a scalar or null is a wrong-kind traversal.

Manifest loading compiles the pointer, validates its leading slash and tilde
escapes, and retains its decoded tokens before resource loading or skip
evaluation. Container kinds, member presence, and array bounds are evaluated
after a selector resolves. Existing `json_assert`, result-value, and diagnostic
paths keep their current dotted grammar; this new surface does not silently
reinterpret checked-in paths.

`missing = true` succeeds only when traversal reaches an absent object member
or an out-of-range canonical array index, including at an intermediate step.
It fails when the complete pointer resolves, even to null, false, or an empty
value. It also fails as invalid traversal when an array token is malformed or
a token is applied to the wrong JSON kind. For example, `/a/b` is absent in
`{}`, wrong-kind in `{ "a": 1 }`, and absent in `{ "a": [] }` when the
pointer is `/a/9/b`. A missing or ambiguous message selector remains a
selector failure and never satisfies path absence.

Path diagnostics identify the assertion and selector, authored pointer,
zero-based token position, resolved prefix, and the specific missing member,
array bound, invalid index, or encountered JSON kind. They do not dump the
complete message.

## Acceptance Model

The manifest syntax rows formerly tracked here are implemented and now live in
[Toolchain Test Harness](../reference/toolchain-test-harness.md#manifest-value-syntax).
The rows below describe planned evidence; they do not imply that the remaining
behavior is implemented.

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Reference root and nested files with portable ASCII components. | Validation returns the authored component sequence and resolves the same spelling in the discovered and copied fixture. | Pure grammar cases plus discovered/copied lookup integration cases. |
| Use an empty reference, a leading, trailing, or repeated slash, or a `.` or `..` component. | Manifest loading reports a portable-reference error before filesystem lookup. | Separator and dot-component rejection matrix. |
| Use backslashes, a drive-relative or drive-absolute spelling, UNC spelling, or a device-path prefix. | Every host reports the same portable-reference error. | Host-independent Windows-spelling matrix. |
| Use a space, colon, control, non-ASCII character, other forbidden punctuation, or a component ending in a period. | Manifest loading rejects the first invalid component. | Component-character boundary matrix. |
| Use a reserved device stem in mixed case, with or without an extension. | The component is rejected; nearby non-reserved stems remain valid. | Portable reserved-name matrix. |
| Reference an existing entry with different ASCII letter case. | Exact-spelling resolution rejects the reference even on a case-insensitive filesystem. | Synthetic exact-name traversal case. |
| Encode a forbidden separator, dot component, control, or reserved stem with a manifest escape. | Validation rejects the decoded reference exactly as it rejects the direct spelling. | Decode-then-validate paired cases. |
| Reference a nested regular file through ordinary directories. | The discovered bytes are snapshotted and the copied fixture contains an independent regular-file counterpart. | Nested sidecar and copy integration case. |
| Reference a final or intermediate symbolic link whose target is inside or outside the fixture. | Loading rejects the first link without reading or copying its target. | No-follow link boundary matrix. |
| Put an unreferenced link, reparse point, or non-regular entry in the discovered case tree. | The copy walk rejects the fixture before command execution. | Whole-tree copy classification cases. |
| Encounter a Windows file symlink, directory symlink, junction, mount point, or other reparse point. | The entry is rejected under the same link-like category. | Windows reparse classifier and supported filesystem integration cases. |
| Let the command overwrite, delete, or replace a copied sidecar with a link. | Invocation and assertion operands continue to use their pre-command discovered snapshots. | Copied-tree mutation and repeated-invocation cases. |
| Reference a regular hard link in an otherwise quiescent fixture. | It is treated as a regular file and its loaded bytes are snapshotted. | Hard-link boundary case and explicit concurrent-mutation non-goal. |
| Reference one valid file from multiple file-backed operands and repeated `$case_text` directives. | The loader reads the distinct path once and every consumer observes the same complete snapshot. | Counting-reader cache cases. |
| Give a skipped case a missing, unreadable, non-UTF-8, or invalid referenced file. | Resource loading fails before skip evaluation; no project is copied and no command starts. | Skip-versus-resource failure matrix with lifecycle sentinels. |
| Load a valid case whose skip rule matches. | Its references are validated and snapshotted, then the case skips without project or command setup. | Valid skipped-case lifecycle case. |
| Fail a later reference after an earlier reference loaded successfully. | No resolved case or partial invocation is produced, and the failure identifies the first invalid field and path in source order. | Transactional load and deterministic-order cases. |
| Run a repeated case after its command overwrites, deletes, or links copied files named by `stdin_file`, `equals_file`, or `$case_text`. | Every repeat uses the original invocation, expected, and substitution snapshots. | Repeated-run mutation integration cases. |
| Compare a command-produced file with an `equals_file` operand. | The actual file is read from the copied project after execution; the expected value remains the discovered snapshot. | Actual-versus-expected authority boundary case. |
| Load plain text sidecars with a byte-order mark, LF, CRLF, a lone CR, and with or without a final line break. | Every file-backed operand observes the original valid UTF-8 bytes without normalization. | Byte-constructed sidecar matrix across all operand roles. |
| Change only the byte-order mark, LF-versus-CRLF spelling, or final line break between an actual and `equals_file`. | Exact comparison reports a mismatch. | Exact text boundary matrix. |
| Load a sidecar containing invalid UTF-8. | Eager resource loading identifies the field and path and fails before skip or command execution. | Invalid UTF-8 lifecycle case. |
| Parse equivalent structured JSON fixture documents with LF, CRLF, mixed layout newlines, and optional final newlines. | Every form produces the same decoded messages and serialized frames. | Structured-fixture layout matrix. |
| Prefix a structured JSON fixture with a UTF-8 or UTF-16 byte-order mark. | Fixture loading rejects the document before command execution. | JSON document encoding failure cases. |
| Substitute `$case_text` containing a byte-order mark, CRLF, non-ASCII text, and a final line break. | The decoded message string preserves every character and `Content-Length` equals the serialized UTF-8 body length. | Substitution and framing byte cases. |
| Query repository attributes for ordinary and `.raw` fixtures under both discovery roots. | Ordinary fixtures are checked out with LF and `.raw` fixtures preserve repository bytes. | Attribute policy checks and representative byte fixtures. |
| Enumerate every presence mask for `stdin`, `stdin_file`, and `stdin_jsonrpc_file`, including empty values. | The empty mask and each singleton are valid, subject to the LSP guard; every pair, triple, or duplicate is rejected. | Generated root-input decision table. |
| Enumerate every presence mask for `equals`, `equals_file`, and `missing` in JSON and result-value assertions. | Exactly one true operation is valid; `missing = false`, omissions, duplicates, and combinations fail before resource loading. | Generated shared operation table. |
| Use `equals_file` containing empty, numeric-looking, null-looking, object-looking, or multiline text against string and non-string selected values. | A string compares as exact text; a non-string reports a kind mismatch and is never coerced or reparsed. | Assertion kind-boundary table. |
| Enumerate omission, `equals`, `equals_file`, both, and unsupported operations in a file assertion. | Exactly one exact text expectation is valid, including an explicitly empty value. | File-assertion decision table. |
| Configure raw `equals_file` alone and with format, positive-fragment, and negative-fragment checks. | Every combination is valid and every configured check is independently required to pass. | Raw stdout and stderr conjunction cases. |
| Enumerate LSP selector masks and operation masks, including occurrence guards and empty string operands. | Exactly one selector and one operation are valid; occurrence is accepted only for a method selector. | Generated LSP selector-by-operation decision table. |
| Use `missing = true` with an existing selected value and missing path, an existing path, and a missing LSP message. | Only the existing selection with a missing path succeeds. | Missing-path versus selector-failure cases. |
| Combine an invalid choice with a missing file operand and a matching skip rule. | Cardinality fails before file loading or skip evaluation. | Validation-order sentinel case. |
| Discover ordinary case directories under both roots. | Execution generation, migration inventory, and policy validation consume the same deterministically sorted root-qualified descriptor list. | Shared-inventory consumer and generated-versus-current parity cases. |
| Mark a discovered case skipped or unavailable on the current host. | Its descriptor remains in resource and policy validation; only command execution skips. | Skipped and non-skipped membership pair. |
| Put `case.toml` below an ancestor case or directly in a configured discovery root. | Discovery fails without producing a partial inventory and identifies the move or removal needed to preserve one visible case boundary. | Nested-manifest and root-shadowing cases. |
| Put a file or directory link, broken link, Windows reparse point, or link cycle anywhere in a discovery root. | Discovery fails at the entry without following or silently omitting it. | No-follow discovery matrix using the shared link classifier. |
| Configure identical or ancestor-related discovery roots. | Discovery rejects overlapping ownership before walking either root. | Root-ownership decision table. |
| Create cases in different filesystem enumeration orders or with equal relative suffixes under different roots. | Descriptor order is stable and the root-qualified identities remain distinct. | Synthetic ordering and two-root collision cases. |
| Add, remove, or relocate a valid case after generated inventory was produced. | Generated-versus-current parity fails with the changed paths and directs the maintainer to rebuild before rerunning. | Inventory parity and build-rerun coverage. |
| Put short or Unicode LF/CR escapes in basic manifest strings at scalar and array positions. | Policy reports the escape-produced line break at its source span. | TOML escape-provenance matrix. |
| Put short or Unicode LF/CR escapes in nested JSON string keys and values. | Policy reports every JSON string occurrence without treating layout whitespace as content. | Nested JSON token matrix. |
| Spell encoded line breaks through literal strings, even and odd backslash runs, or Unicode-assembled backslash and marker characters. | The decoded-spelling rule rejects every equivalent single-token form. | Generated parity and obfuscation matrix. |
| Put physical LF, CRLF, or mixed newlines in multiline strings, including folded continuations. | The policy accepts physical structure while the manifest grammar supplies the defined decoded value. | Physical-newline and folding negative controls. |
| Put the same forbidden-looking text in a comment, a string, and a sidecar. | Only the string token is a policy finding; comments and sidecars remain outside the gate. | Lexical-region and scope boundary cases. |
| Split a backslash and marker across separate strings, or use a numeric, entity, percent, or non-LF Unicode encoding. | The policy does not infer concatenation or application-specific decoding. | Token-boundary and false-positive matrix. |
| Put an invalid escape in a string. | Manifest grammar reports its specific syntax failure instead of reclassifying it as a policy finding. | Parser-before-policy precedence cases. |
| Render a policy finding in harness output. | The message shows the field, location, spelling, replacement action, and readability reason. | Human-output rendering case. |
| Put multiple findings and lexical scan errors in multiple authoritative manifests. | One preflight failure reports every reliable finding in deterministic order and produces no harness test binary. | Multi-file aggregation and build-generation cases. |
| Put a finding in a skipped or unavailable-tool case. | Global preflight fails before resource or skip evaluation and no command starts. | Skipped-case preflight sentinel. |
| Enumerate the same invalid tree in different filesystem orders. | Aggregate report bytes and totals are identical. | Shuffled-entry determinism case. |
| Start generated cases concurrently while runtime preflight is pending. | One scan runs, all case threads wait, and no command starts unless the shared result succeeds. | Latch-based barrier concurrency case. |
| Run a prebuilt binary after adding, removing, or relocating a manifest. | Runtime parity fails with the changed paths and no generated case command starts. | Stale-inventory decision table and spawn sentinel. |
| Run an inventory-external synthetic case with a violation. | The local guard rejects it before referenced-file loading, skip evaluation, or command execution. | Synthetic local-policy cases. |
| Run the clean unfiltered harness target. | Build and runtime preflights use the same scanner, the repository policy route observes zero findings, and normal parallel execution proceeds. | Shared-scanner call-count and complete harness cases. |
| Generate the semantic baseline twice from the same pre-migration tree and schema. | Both outputs, case ordering, and aggregate digests are byte-identical. | Deterministic baseline exporter case. |
| Rewrite escaped inline text as byte-equivalent multiline or file-backed text. | The semantic descriptor remains identical, including exact final-line-break, BOM, CRLF, and indentation bytes. | Representation-equivalence matrix. |
| Change an invocation field, assertion kind, fragment, typed JSON value, selector, path, ordered sequence, or exact byte. | The comparator rejects the precise logical component and shows useful old/new metadata; object-member order alone remains semantically equal. | Field-level mutation and weakening matrix. |
| Add, remove, or rename a discovered case during the migration. | Case-set comparison fails before field comparison; count-preserving substitution does not pass. | Inventory mutation cases. |
| Convert raw JSON-RPC requests to equal ordered decoded messages with different serialization. | The change passes only through an exact typed request-conversion record; message value or sequence-order changes fail. | Structured-request conversion cases. |
| Replace raw LSP output evidence with decoded assertions without a record, then with a fully fingerprinted mapping. | The unapproved change fails; the mapping consumes only its listed old and new assertions. | Raw-to-decoded conversion matrix. |
| Apply a conversion record to a framing-specific case or add an unrelated change to an approved case. | The record is rejected and raw framing evidence or the unrelated contract must be restored. | Non-blanket approval cases. |
| Use a wildcard, stale fingerprint, overlapping mapping, missing assertion mapping, or unused conversion record. | Conversion-record validation fails before equivalence is reported. | Override-abuse and consumption matrix. |
| Compare every migrated descriptor and run the full harness. | Every baseline case has one unchanged or explicitly converted successor, no unexpected difference remains, and runtime behavior passes independently. | Repository-wide comparator plus unfiltered harness target. |
| Supply requests with string, numeric, or null identifiers and a notification without `id`. | Every valid envelope is framed in fixture order; absence of `id` alone defines a notification. | Request and notification envelope matrix. |
| Omit `params` or use object, array, or null parameters. | The structured profile accepts the envelope without method-specific validation. | Parameter-kind acceptance cases and representative shutdown migration. |
| Omit or mistype `jsonrpc` or `method`, use another version, or use an unsupported `id` or `params` kind. | Fixture loading identifies the indexed member failure before command execution. | Envelope member failure matrix with spawn sentinel. |
| Include `result` or `error` in a client-input element. | Structured loading rejects the response-shaped object. | Response and mixed-shape rejection cases. |
| Use an unknown method, extension member, repeated identifier, or method-specific invalid parameter object. | The harness preserves and frames it so server behavior remains observable. | Envelope-validator non-goal cases. |
| Put an invalid element after valid elements. | Loading fails transactionally and the command receives no partial structured input. | Late-element atomicity case. |
| Send an invalid envelope, client response, batch body, malformed JSON body, or transport defect through raw input. | The exact bytes reach the server without structured-envelope validation. | Structured-rejection and raw-routing pairs. |
| Parse valid integer, fraction, exponent, negative-zero, large-magnitude, and high-precision numbers. | Each value is accepted without range loss and reserializes with its original number lexeme. | RFC number grammar and lossless round-trip matrix. |
| Parse a leading plus or zero, incomplete fraction or exponent, NaN, or infinity. | JSON loading rejects the invalid number at its source location. | Invalid number grammar matrix. |
| Compare exponent, trailing-zero, and signed-zero spellings of equal decimals, then nearby unequal large values. | Equal mathematical decimals compare equal without rounding; unequal values and strings remain distinct. | Arbitrary-precision decimal equality table. |
| Decode direct Unicode, every JSON escape, BMP escapes, and valid high-low surrogate pairs. | Equivalent spellings produce the same Unicode scalar sequence. | JSON string and Unicode decoding matrix. |
| Decode an isolated, reversed, truncated, or mismatched surrogate escape. | Fixture or framed-message decoding fails before request framing or assertion selection. | Surrogate failure matrix. |
| Repeat an object member name literally or through an escape-equivalent spelling at any depth. | JSON decoding rejects the second decoded name and identifies its object context. | Nested duplicate-name matrix. |
| Compare objects with equal unique members in different orders and arrays with reordered elements. | Object equality succeeds and array equality fails. | Object-versus-array semantic equality cases. |
| Serialize an object whose authored order differs from key order and whose numbers use non-canonical spellings. | The body retains member and number-lexeme order while `Content-Length` matches the emitted UTF-8 bytes. | Structured serialization capture case. |
| Parse the same value corpus as a structured fixture and as framed stdout. | Both routes produce identical accept, reject, and semantic-value results. | Shared JSON-profile conformance table. |
| Put `$case_text` directives in object values and array elements at several depths. | Each complete directive becomes the exact sidecar string while surrounding member and element order remains unchanged. | Recursive transformation table. |
| Put `$case_text` or `$case_literal` in an outer message-list element. | Transformation occurs, then only an object result can pass envelope revalidation. | Post-transformation root-kind pairs. |
| Give a reserved directive a non-string path, an extra member, both reserved members, or another malformed shape. | Fixture loading identifies the JSON value position and fails before reading sidecars or starting a command. | Reserved-shape matrix with lifecycle sentinels. |
| Spell a reserved key directly and with an escape-equivalent decoded name. | Both have identical directive meaning; duplicate decoded names fail before transformation. | Decoded-key recognition cases. |
| Insert text that contains valid or invalid JSON, directive spelling, another path, BOM, or CRLF. | It remains one exact JSON string and triggers no recursive parsing, path lookup, or substitution. | Non-chaining substitution and byte-preservation cases. |
| Wrap a literal `$case_text` application object, a literal `$case_literal` object, or a reserved-looking subtree in `$case_literal`. | The payload appears unchanged and no directive inside it is processed. | Literal-barrier expressibility matrix. |
| Reference the same path from multiple directives and other operands. | One immutable per-case snapshot supplies every consumer and transformation position. | Counting-reader cache integration case. |
| Fail a late directive or the post-transformation envelope check. | The structured fixture produces no partial frame and the command does not start. | Transactional late-failure cases. |
| Convert raw frames to a structured fixture with equal decoded messages but different whitespace, object order, string escapes, number spelling, or incidental header presentation. | Semantic comparison succeeds only through a scoped conversion record that lists the discarded wire dimensions. | Before-and-after semantic conversion matrix. |
| Change message count or order, a member value or type, array order, string content, or an extension member. | Semantic comparison rejects the migration. | Ordered message mutation matrix. |
| Serialize ASCII, multi-byte Unicode, controls, quotes, backslashes, a byte-order mark in string data, and `$case_text` CRLF. | Body bytes follow the deterministic escape policy and header length equals their exact UTF-8 byte count. | Serializer and framing golden matrix. |
| Serialize multiple messages from LF- and CRLF-formatted fixture documents and repeat the invocation. | Every form and repeat produces the same directly concatenated canonical frames. | Multi-frame determinism case. |
| Fail validation or serialization for a later message. | No command starts and no prefix of the framed sequence is sent. | All-materialized-before-start sentinel. |
| Attempt semantic conversion of malformed, partial, conflicting-length, invalid-JSON, or spelling-sensitive raw input. | Conversion is rejected and the case retains exact raw input. | Nonconvertible transport matrix. |
| Replay baseline raw and generated structured input against the same LSP binary. | Exit, stderr contract, and relevant decoded responses agree; presentation sensitivity keeps the case raw. | Differential migration replay cases. |
| Select minimum, maximum, ordinary, signed-zero, string, empty-string, Unicode-string, and null response IDs. | Each selector matches only the same decoded identifier kind and value. | Identifier-domain success matrix. |
| Select `1`, `1.0`, `1e0`, and other in-range integral decimal spellings. | Every spelling denotes the same numeric identifier without floating-point conversion. | Numeric identifier equivalence table. |
| Compare numeric `1`, string `"1"`, null, string `"null"`, and an absent `id`. | All typed identities and absence remain distinct. | Cross-kind and presence matrix. |
| Use a non-integral, out-of-range, boolean, array, or object selector. | Manifest validation identifies `lsp_assert.id` and fails before command execution. | Selector-domain failure table. |
| Decode a response with an unsupported ID kind or value. | Message classification reports an invalid response identifier before assertions run. | Decoded-response ID failure matrix. |
| Decode numerically equivalent IDs with different lexemes in one stream. | They enter the same identifier group for duplicate handling. | Identifier-to-cardinality grouping cases. |
| Enumerate the presence masks for `method`, `id`, `result`, and `error`, including present null values. | Only the response, notification, and server-request rows classify; every other combination reports a stable envelope failure. | Generated classification decision table. |
| Decode success responses, valid error responses, and notifications with each allowed parameter shape. | Each message enters exactly one supported class before selectors run. | Response, error-object, and notification matrix. |
| Decode a response with both or neither result/error, params, a missing ID, or malformed error object. | Classification rejects the specific response-envelope fact. | Response failure matrix. |
| Decode a notification with an ID, result/error, non-string method, or scalar/null parameters. | It is classified as another complete shape or rejected; it is never silently treated as a notification. | Notification boundary matrix. |
| Emit a server request whose ID or method would otherwise match an assertion. | Decoded mode fails as unsupported before any selector comparison and the request enters no candidate set. | Unsupported-server-request bait case. |
| Give a response and request the same ID, or a notification and request the same method. | Only the supported response or notification is a selector candidate, while the request independently fails the stream. | Class-isolation cases. |
| Put a late invalid or unsupported message after valid candidates. | Full classification fails transactionally and no earlier assertion is reported as passing the case. | Late-classification failure case. |
| Keep the same server-request or invalid-message bytes in a raw-output-only case. | Raw stream assertions remain available without semantic classification. | Raw-versus-decoded mode routing pair. |
| Emit selected or unselected success and error responses with the same non-null ID. | Global cardinality fails before path assertions and identifies every message index in the duplicate group. | Response duplicate matrix. |
| Emit numeric-equivalent response IDs, signed-zero variants, and numeric-versus-string controls. | Numerically equivalent IDs collide, while different JSON kinds remain distinct. | Identifier grouping matrix. |
| Emit zero, one, or several null-ID responses and select or do not select null. | Null responses do not violate the global rule; a null selector reports missing, succeeds, or reports ambiguity for the three cardinalities. | Null non-correlation table. |
| Apply several assertions to the same unique ID or method occurrence. | Every assertion reuses the same message without consuming or advancing the selection. | Multi-path selector reuse cases. |
| Emit interleaved notifications of several methods and select omitted, first, middle, last, and out-of-range occurrences. | Only same-method notifications count in frame order; valid indices allow extras and an invalid index reports the observed count. | Notification occurrence decision table. |
| Repeat an invocation that emits the same identifiers and methods. | Each invocation computes cardinality independently. | Repeat isolation case. |
| Put a late duplicate response after messages that would satisfy every assertion. | Global validation fails before any partial assertion success determines the case outcome. | Late-duplicate lifecycle case. |
| Decode canonical frames with optional valid Content-Type, varied header case and order, and leading-zero lengths. | Every protocol-equivalent form yields the same body object. | Accepted header-variation matrix. |
| Use invalid colon spacing, LF-only or mixed separators, bare CR, or header folding. | Framing fails at the first invalid header byte and no selector runs. | Header syntax and line-ending matrix. |
| Omit, duplicate, conflict, sign, overflow, or otherwise malform Content-Length. | Framing reports the specific length fact before slicing or allocating from it. | Content-Length cardinality and grammar table. |
| Use absent, canonical, case-varied, or legacy UTF-8 Content-Type, then another media type, charset, parameter, duplicate, or unknown header. | Only the supported UTF-8 profile succeeds. | Content-Type and header-set decision table. |
| Count ASCII, multi-byte Unicode, non-BMP, quote, control, and header-looking body text. | The decoder uses the declared byte boundary and never scalar counts or resynchronization scans. | Byte-versus-scalar and opaque-body cases. |
| Supply invalid UTF-8, a body byte-order mark, malformed JSON, a second JSON value, or a non-object root. | The decoder reports the frame's encoding, JSON, or root failure before classification. | Body validation matrix. |
| Concatenate valid frames with no separator and vary later-frame validity. | Bodies retain exact boundaries and a later failure identifies its frame index and absolute stdout offset. | Multi-frame and no-resynchronization cases. |
| Feed canonical structured-writer output to the decoder and pair malformed bytes with raw assertions. | Writer output always decodes; raw mode can observe forms semantic mode rejects. | Writer-decoder conformance and raw-routing pairs. |
| Exit with each expected and actual zero, nonzero, and signal outcome while raw and semantic checks are configured. | Only a status mismatch fails the exit check, and every capturable stream check still runs. | Exit-by-check dependency table. |
| Satisfy an early selector, then end in a partial header or body or append a trailing byte. | Raw checks run, semantic preflight reports one root failure at the incomplete frame or trailing offset, and no assertion uses the valid prefix. | Partial and trailing complete-sequence matrix. |
| Produce empty stdout with an LSP assertion. | Transport preflight succeeds with zero messages and the assertion reports its missing selector. | Empty-versus-partial boundary case. |
| Use the same partial or malformed bytes in a raw-only case and a decoded-assertion case. | Raw checks alone decide the first case; the second also reports the semantic prerequisite failure. | Raw-versus-semantic routing pairs. |
| Fail raw stdout while the semantic stream is valid, then fail a decoded assertion while raw stdout is valid. | The two consumers remain independent and neither failure suppresses the other. | Raw and semantic independence truth table. |
| Decode a valid complete stream with several failing LSP assertions. | Every selector, path, and comparison failure is collected in manifest order. | Multi-assertion accumulator case. |
| Run three repeats whose middle or later runs differ in exit, framing, and selected value. | Every capturable invocation runs, state restarts per run, and findings are grouped by run and fixed check order. | Scripted repeat aggregation and isolation case. |
| Fail process launch or reliable capture rather than returning an ordinary child result. | One harness-infrastructure failure stops unsafe remaining repeats and identifies how many were not attempted. | Injectable process-runner failure table. |
| Select the message root, empty key, dotted key, numeric object key, slash or tilde key, space, and Unicode key with JSON Pointers. | Every token resolves by exact decoded scalar spelling, and the empty pointer selects the complete message. | RFC 6901 pointer and object-key matrix. |
| Decode `~0`, `~1`, `~01`, and adjacent escapes, then use an invalid tilde escape, a nonempty path without `/`, or a URI fragment. | Valid escapes decode once in RFC order; invalid pointer syntax fails during manifest loading before resources or command execution. | Pointer syntax and escape-order table. |
| Apply `/0` to an object and an array, then use canonical first, last, length, and huge indexes. | The object selects key `0`; the array selects by index; length and huge indexes are reported as absent without overflow. | Numeric-key and array-boundary cases. |
| Use a leading-zero, signed, whitespace, non-ASCII, or `-` token while traversing an array. | Every operation reports invalid array traversal, and `missing = true` does not pass. | Invalid array-token matrix. |
| Resolve an absent member, out-of-range index, present null, and a token through a scalar under equality and `missing = true`. | Only member or index absence satisfies `missing`; present and wrong-kind results remain distinct failures. | Path-resolution truth table. |
| Compare `{}`, `{ "a": 1 }`, and `{ "a": [] }` with deeper pointers. | Intermediate absence succeeds for `missing`, while scalar traversal reports the encountered wrong kind. | Intermediate absence-versus-type cases. |
| Use `missing = true` after a missing or ambiguous selector. | Selector resolution fails before pointer traversal; path absence cannot rescue it. | Selector-to-path dependency case. |
| Run every existing general JSON/result path plus new LSP pointers. | Existing dotted-path results remain unchanged and every LSP case uses only the pointer grammar. | Repository path-compatibility and LSP pointer inventory cases. |
| Run representative migrated exact-output, fragment, formatting, repeated-run, and LSP cases. | Their observable checks match the pre-migration cases. | Focused migrated toolchain cases. |
| Discover any `case.toml` containing a policy-prohibited encoded LF, CR, or literal escape spelling. | Global preflight fails before any case command and identifies the token, location, and spelling. | Repository-wide token-policy scanner with a synthetic failing inventory. |
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
3. No authoritative `case.toml` string token contains an escape-produced LF or
   CR or a decoded spelling that the encoded-line-break predicate prohibits.
4. The shared build preflight and runtime barrier scan the authoritative
   inventory before any generated case command. The harness-owned policy test
   verifies the same scanner and inventory through the existing target; no
   separate workflow or `AGENTS.md` rule is required.
5. The complete toolchain harness suite passes after migration.
6. The implemented behavior is promoted to the matching pages under
   `../specification/` and executable examples under
   `../../examples/specification/`. The supporting harness reference is
   updated in `../reference/toolchain-test-harness.md`.
7. The completed proposal is removed from `docs/proposals/` and its durable
   rationale or completion evidence is retained under
   `../reference/implemented-proposals/` only if it remains useful.

The zero-escape scan is necessary but not sufficient. It proves the textual
migration target, while the parser, sidecar, JSON-RPC, semantic-equivalence,
and full-suite evidence prove that moving the content did not weaken or change
the tests.

## Implementation Slices

1. Add the shared lossless manifest lexer, multiline string and array parser,
   canonical discovery inventory, and build/runtime policy preflight without
   migrating cases.
2. Add portable case-file operands, no-follow copy checks, eager immutable
   snapshots, repository line-ending attributes, and boundary coverage.
3. Add the JSON interoperability profile, structured request transformation
   and framing, decoded LSP transport and assertions, JSON Pointer paths, and
   dependency-aware result aggregation.
4. Migrate representative cases from every operand and assertion shape,
   recording only narrowly reviewed LSP conversions and differential replay
   evidence.
5. Migrate both discovery roots, require zero policy findings and zero
   unexplained semantic differences, then run the complete harness target.
6. Promote implemented behavior to specification and executable examples,
   update the harness reference, remove migration-only artifacts that no longer
   have value, and complete the proposal lifecycle audit.

## Guardrail Placement

The regression risk is a deterministic property of harness-owned manifests.
The narrowest effective guardrail is therefore the shared harness discovery
and token-policy implementation: build preflight prevents generation from a
dirty inventory, the runtime barrier protects prebuilt binaries, and the
harness test verifies the same scanner. A prose-only convention would not
protect human and automated contributors equally. A new CI job would duplicate
the existing target, and an always-on repository instruction would expose a
harness-specific rule to unrelated work.
