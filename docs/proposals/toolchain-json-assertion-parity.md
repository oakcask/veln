---
role: proposal
update-when: The proposed shared toolchain JSON assertion operations, compatibility boundary, acceptance evidence, or implementation status changes.
---

# Toolchain JSON Assertion Parity

## Summary

Give every toolchain harness assertion that selects a JSON value the same
operation surface. The affected sections are `[[json_assert]]`,
`[[result_value_assert]]`, `[[lsp_assert]]`, and `[[mcp_assert]]`.

Each section keeps its current input source, selector, and path syntax. After
the section selects a JSON value, it supports the same equality, file-backed
equality, string containment, array length, workspace-file URI, and missing
value operations.

JSON object member order is not observable through these assertions. JSON
array element order and length remain observable.

## Motivation

The toolchain harness currently divides JSON operations by command transport.
MCP assertions can check array length and canonical workspace file URIs, while
other JSON-valued assertions cannot. JSON and result-value assertions can load
complete expected JSON from a case file, while MCP and LSP assertions cannot.
LSP assertions can check string containment, while the other sections cannot.

This division makes fixture authors choose raw output checks or duplicate
larger values when the required operation is absent from the section that owns
the selected value. It also gives JSON object equality different meanings in
different sections.

No checked fixture intentionally treats JSON object member order as behavior.
The existing JSON fixtures that contain objects depend on insertion order only
because their assertions use the current structural representation. Arrays do
carry ordered evidence and must retain ordered comparison.

## Current Boundary

[Toolchain Test Harness](../reference/toolchain-test-harness.md) specifies the
implemented assertion behavior. The current operation split is:

| Section | Selected input | Current operations | Current equality boundary |
| --- | --- | --- | --- |
| `[[json_assert]]` | Parsed JSON command stdout and a dot-separated path. | `equals`, `equals_file`, `equals_json_file`, `missing` | Object member order affects equality. A complete inline decimal or exponent number is rejected. |
| `[[result_value_assert]]` | A parsed rendered Veln result value and a dot-separated path. | `equals`, `equals_file`, `equals_json_file`, `missing` | Object member order affects equality. A complete inline decimal or exponent number is rejected. |
| `[[lsp_assert]]` | One selected LSP response or notification and an RFC 6901 JSON Pointer. | `equals`, `equals_file`, `contains`, `missing` | Object member order affects equality. A complete inline decimal or exponent number is rejected. |
| `[[mcp_assert]]` | One selected MCP response and an RFC 6901 JSON Pointer. | `equals`, `length`, `workspace_file_uri`, `missing` | Object member order does not affect equality. Decimal and exponent number spellings are preserved. |

The semantic baseline already key-sorts JSON objects because object member
order is not part of an assertion value. It retains array order and assertion
section order.

## Proposed Contract

### Common Operation Surface

Each affected assertion declares exactly one operation from this table.

| Operation | Operand | Required selected value | Required result |
| --- | --- | --- | --- |
| `equals` | One inline JSON value. | Any JSON value. | Compare the complete selected value using the JSON equality rules below. |
| `equals_file` | One immutable case-relative text file. | A JSON string. | Compare the selected string with the exact file contents without parsing the file as JSON. |
| `equals_json_file` | One immutable case-relative text file containing one JSON value. | Any JSON value. | Parse the file as JSON and compare the complete selected value using the JSON equality rules below. |
| `contains` | One inline string. | A JSON string. | Require the selected string to contain the operand as one contiguous substring. |
| `length` | One non-negative integer representable by the harness collection length type. | A JSON array. | Require the selected array to contain exactly the stated number of elements. |
| `workspace_file_uri` | One safe workspace-relative file path. | A JSON string. | Compare the selected string with the canonical `file:` URI of that copied workspace file. |
| `missing = true` | The boolean value `true`. | No selected value. | Succeed only when the assertion selector exists and the assertion path is missing. |

An omitted operation, more than one operation, or `missing = false` is a
manifest error. A file-backed operand is read from the immutable discovered
case, not from a file changed by the command under test.

### JSON Equality

Equality compares JSON values recursively.

| JSON kind | Equality rule |
| --- | --- |
| Null, boolean, or string | Both values have the same kind and value. |
| Number | Both values have the same JSON number spelling. `1`, `1.0`, and `1e0` are distinct. |
| Array | Both arrays have the same length, and each element at an index is equal. |
| Object | Both objects contain equal member-name and member-value pairs. Member order does not affect equality. |

Every affected section accepts an inline decimal or exponent number for
`equals`. The harness preserves its JSON spelling for comparison and semantic
baseline output.

An object comparison does not expose serializer insertion order. An array
comparison continues to expose element order. Reversing an array with two
distinct elements fails equality.

### Selection And Path Compatibility

This proposal does not change how an assertion obtains its selected value.

| Section | Selector and path retained by this proposal |
| --- | --- |
| `[[json_assert]]` | Select parsed JSON stdout, then use the existing dot-separated path. |
| `[[result_value_assert]]` | Select and parse the string at `value_path`, then use the existing dot-separated result-value path. |
| `[[lsp_assert]]` | Select one response by `id` or one notification by `method` and `occurrence`, then use an RFC 6901 JSON Pointer. |
| `[[mcp_assert]]` | Select exactly one response by string or integer `id`, then use an RFC 6901 JSON Pointer. |

A missing selector is different from a missing path. `missing = true` can
satisfy only a missing path after selection succeeds. Invalid LSP or MCP
pointer traversal remains an assertion failure and does not satisfy
`missing = true`.

### Workspace File URI Safety

`workspace_file_uri` keeps the current MCP safety boundary in every affected
section. Its operand must identify an existing regular file below the copied
case workspace. It rejects absolute paths, empty paths, `.`, `..`, empty path
segments, backslashes, links, link-like components, non-file entries, and
canonical paths outside the workspace.

The comparison uses the canonical URI spelling produced for the copied
workspace. It does not expose or store the temporary workspace path in the
manifest or semantic baseline.

### Failure Reporting

An operation failure identifies the case run, assertion section, assertion
index or selector where applicable, selected path, and failed fact. The failed
fact distinguishes at least these outcomes:

| Failure | Required fact in the failure |
| --- | --- |
| A selected path is absent for an operation other than `missing`. | The selected JSON path was not found. |
| A selected path exists for `missing = true`. | The selected JSON path exists but should be missing. |
| `contains`, `equals_file`, or `workspace_file_uri` selects a non-string. | The operation requires a JSON string. |
| `length` selects a non-array. | The operation requires a JSON array. |
| `equals` or `equals_json_file` differs. | The complete expected and actual JSON values differ. |
| `length` differs. | The expected and actual array lengths differ. |
| `workspace_file_uri` differs. | The expected and actual URI strings differ. |

Transport decoding and selector failures retain their section-specific
context. The common operation contract does not replace LSP framing errors,
MCP JSONL errors, or response and notification selection errors.

## Compatibility And Migration

Every currently valid assertion manifest remains valid. Existing selectors,
paths, and operations keep their current spelling.

Two comparisons become less dependent on representation:

- Reordered JSON object members become equal in every affected section.
- Complete inline decimal and exponent numbers become valid `equals` operands
  outside MCP.

Array ordering remains strict. Existing array assertions therefore retain
their ordering evidence. The implementation does not require a bulk rewrite
of existing fixtures. Representative fixtures adopt newly available
operations to provide executable evidence for each assertion source.

The checked semantic baseline records the operation and typed operand for each
assertion. Its object values remain key-sorted. Its arrays and assertion lists
remain ordered.

## Non-Goals

- Replace the dot-separated paths used by JSON and result-value assertions
  with JSON Pointer paths.
- Change LSP or MCP transport decoding and message selection.
- Add these JSON operations to `[[diagnostics]]` or `[[file_assert]]`.
- Treat JSON object member order as an observable serialization contract.
- Ignore JSON array element order.
- Add a general JSON query, predicate, or matching language.
- Convert every raw stdout fixture to a decoded assertion.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Parse the common operation surface in each affected section. | Each section accepts exactly one of the seven operations and rejects omission, duplication, `missing = false`, and invalid operand types. | Table-driven manifest parser success and rejection tests in the toolchain harness. |
| Compare reordered objects. | Equal objects pass in JSON, result-value, LSP, and MCP assertions when only member order differs. | Shared evaluator tests exercised through all four assertion adapters. |
| Preserve ordered arrays. | Equal arrays pass. A reversed array with distinct elements fails. An independently declared `length` checks exact cardinality. | Shared equality and length tests plus representative JSON-valued assertion fixtures. |
| Compare JSON number spellings. | Each section accepts decimal and exponent operands. `1`, `1.0`, and `1e0` remain distinct. | Manifest parsing and evaluator number-matrix tests for all four sections. |
| Use file-backed expected values. | Each section supports exact string files and parsed JSON files. Invalid JSON files and wrong selected value kinds fail with operation-specific facts. | Immutable case-file parser tests and evaluator rejection tests. |
| Check selected string containment. | Each section accepts `contains`; a non-string or absent substring fails. | Shared operation tests exercised through all four assertion adapters. |
| Compare a copied workspace file URI. | Each section computes the canonical URI for an existing safe workspace-relative file and rejects unsafe operands and non-string selected values. | Cross-section URI success matrix and the existing link and workspace-escape rejection matrix. |
| Preserve selector and path behavior. | JSON and result-value dot paths, LSP response and notification selection, MCP response selection, and LSP/MCP pointer failures retain their current outcomes. | Existing section-specific tests plus regression rows that use a newly shared operation after selection. |
| Publish representative executable evidence. | A JSON case and a result-value case use `length` or file-backed JSON equality; the LSP publish-diagnostics case checks diagnostic cardinality and its workspace URI; the MCP definition case retains response-local length and workspace URI checks. | Checked cases under `crates/veln-cli/tests/toolchain_cases/` and `examples/specification/`. |
| Keep the semantic inventory reviewable. | The baseline records each common operation and operand, key-sorts objects, and preserves array and assertion order. | Semantic baseline unit tests and the checked authoritative-case baseline comparison. |
| Publish the implemented contract. | Current common behavior and section-specific selection boundaries are documented without relying on this proposal as authority. | Update `../reference/toolchain-test-harness.md` after executable evidence passes. |

## Completion Rule

This proposal completes when every acceptance row has checked evidence, the
normative toolchain harness reference describes the common operation contract,
and representative executable cases use the shared surface. Move the completed
record to `../reference/implemented-proposals/` and remove this page from the
proposal catalog when the implementation is complete.
