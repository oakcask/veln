---
role: proposal
update-when: The proposed shared toolchain JSON assertion operations, compatibility boundary, acceptance evidence, or implementation status changes.
---

# Toolchain JSON Assertion Parity

## Summary

Complete the remaining operation parity for toolchain harness assertions that
select a JSON value. The affected sections are `[[json_assert]]`,
`[[result_value_assert]]`, `[[lsp_assert]]`, and `[[mcp_assert]]`.

The common JSON equality foundation is implemented. The current contract is
specified by [Toolchain Test Harness](../reference/toolchain-test-harness.md).
String containment is implemented for every affected section. This proposal
now covers the remaining file-backed equality, array length, and
workspace-file URI gaps.

## Motivation

The toolchain harness currently divides JSON operations by command transport.
MCP assertions can check array length and canonical workspace file URIs, while
other JSON-valued assertions cannot. JSON and result-value assertions can load
complete expected JSON from a case file, while MCP and LSP assertions cannot.
All four assertions can check string containment. The remaining operation
split still requires fixture authors to choose raw output checks or duplicate
larger values for operations that have not reached every section.

This division makes fixture authors choose raw output checks or duplicate
larger values when the required operation is absent from the section that owns
the selected value. The implemented equality foundation removed the former
section-specific object-order and number-spelling differences without changing
the remaining operation split.

## Current Boundary

[Toolchain Test Harness](../reference/toolchain-test-harness.md) specifies the
implemented assertion behavior. The current operation split is:

| Section | Selected input | Current operations | Current equality boundary |
| --- | --- | --- | --- |
| `[[json_assert]]` | Parsed JSON command stdout and a dot-separated path. | `equals`, `equals_file`, `equals_json_file`, `contains`, `missing` | Uses the common JSON equality contract. |
| `[[result_value_assert]]` | A parsed rendered Veln result value and a dot-separated path. | `equals`, `equals_file`, `equals_json_file`, `contains`, `missing` | Uses the common JSON equality contract. |
| `[[lsp_assert]]` | One selected LSP response or notification and an RFC 6901 JSON Pointer. | `equals`, `equals_file`, `contains`, `missing` | Uses the common JSON equality contract. |
| `[[mcp_assert]]` | One selected MCP response and an RFC 6901 JSON Pointer. | `equals`, `contains`, `length`, `workspace_file_uri`, `missing` | Uses the common JSON equality contract. |

The common equality contract ignores object member order, retains array order
and length, distinguishes JSON kinds and nested values, and preserves complete
number spelling. The semantic baseline key-sorts objects while retaining array
order, assertion order, and JSON number spelling.

## Remaining Proposed Contract

### Remaining Operation Additions

The implemented `equals`, `contains`, and `missing = true` contracts are
specified by [Toolchain Test Harness](../reference/toolchain-test-harness.md).
The remaining work adds only the section and operation pairs in this matrix.

| Section | Remaining operations |
| --- | --- |
| `[[json_assert]]` | `length`, `workspace_file_uri` |
| `[[result_value_assert]]` | `length`, `workspace_file_uri` |
| `[[lsp_assert]]` | `equals_json_file`, `length`, `workspace_file_uri` |
| `[[mcp_assert]]` | `equals_file`, `equals_json_file` |

Each listed operation uses the operand and selected-value contract in this
table.

| Operation | Operand | Required selected value | Required result |
| --- | --- | --- | --- |
| `equals_file` | One immutable case-relative text file. | A JSON string. | Compare the selected string with the exact file contents without parsing the file as JSON. |
| `equals_json_file` | One immutable case-relative text file containing one JSON value. | Any JSON value. | Parse the file as JSON and compare the complete selected value using the current common JSON equality contract. |
| `length` | One non-negative integer representable by the harness collection length type. | A JSON array. | Require the selected array to contain exactly the stated number of elements. |
| `workspace_file_uri` | One safe workspace-relative file path. | A JSON string. | Compare the selected string with the canonical `file:` URI of that copied workspace file. |

The existing exactly-one-operation rule applies as each operation is added. A
file-backed operand is read from the immutable discovered case, not from a
file changed by the command under test.

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
| `equals_file` or `workspace_file_uri` selects a non-string. | The operation requires a JSON string. |
| `length` selects a non-array. | The operation requires a JSON array. |
| `equals` or `equals_json_file` differs. | The complete expected and actual JSON values differ. |
| `length` differs. | The expected and actual array lengths differ. |
| `workspace_file_uri` differs. | The expected and actual URI strings differ. |

Transport decoding and selector failures retain their section-specific
context. The common operation contract does not replace LSP framing errors,
MCP JSONL errors, or response and notification selection errors.

## Compatibility And Migration

Every currently valid assertion manifest remains valid. Existing selectors,
paths, operations, common equality behavior, and the implemented common
`contains` behavior keep their current spelling and meaning. Representative
fixtures must adopt each newly available operation to provide executable
evidence for its assertion source.

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
| Parse the remaining operation additions in each affected section. | Each section accepts each newly added operation with its declared operand type and retains rejection of omission, duplication, `missing = false`, and invalid operand types. | Table-driven manifest parser success and rejection tests in the toolchain harness. |
| Use file-backed expected values. | Each section supports exact string files and parsed JSON files. Invalid JSON files and wrong selected value kinds fail with operation-specific facts. | Immutable case-file parser tests and evaluator rejection tests. |
| Compare a copied workspace file URI. | Each section computes the canonical URI for an existing safe workspace-relative file and rejects unsafe operands and non-string selected values. | Cross-section URI success matrix and the existing link and workspace-escape rejection matrix. |
| Preserve selector and path behavior. | JSON and result-value dot paths, LSP response and notification selection, MCP response selection, and LSP/MCP pointer failures retain their current outcomes. | Existing section-specific tests plus regression rows that use a newly shared operation after selection. |
| Publish representative executable evidence. | A JSON case and a result-value case use `length` or file-backed JSON equality; the LSP publish-diagnostics case checks diagnostic cardinality and its workspace URI; the MCP definition case retains response-local length and workspace URI checks. | Checked cases under `crates/veln-cli/tests/toolchain_cases/` and `examples/specification/`. |
| Keep the semantic inventory reviewable. | The baseline records each newly common operation and operand while preserving the implemented equality baseline. | Semantic baseline unit tests and the checked authoritative-case baseline comparison. |
| Publish the completed operation contract. | Current common operations and section-specific selection boundaries are documented without relying on this proposal as authority. | Update `../reference/toolchain-test-harness.md` after executable evidence passes. |

## Completion Rule

This proposal completes when every acceptance row has checked evidence, the
normative toolchain harness reference describes the common operation contract,
and representative executable cases use the shared surface. Move the completed
record to `../reference/implemented-proposals/` and remove this page from the
proposal catalog when the implementation is complete.
