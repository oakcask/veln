---
role: proposal
update-when: The readable toolchain case stream proposal scope, structured JSON-RPC request fixture validation route, decoded LSP assertion acceptance evidence, migration boundary, or completion gate changes.
---

# Readable Toolchain Case Streams

## Summary

The remaining proposal work is limited to representative LSP case migration
and proposal completion. Implemented decoded LSP response assertions,
structured JSON-RPC request fixtures, manifest multiline syntax, case-relative
text sidecars, portable path checks, discovery inventory, encoded-line-break
policy, build preflight, runtime stale inventory barrier, and checked-in
case-text migration are current harness behavior. Their normative route is
[Toolchain Test Harness](../reference/toolchain-test-harness.md).

Implemented LSP assertions now target decoded responses and notifications when
message structure is the observable behavior. The remaining work migrates
representative cases without weakening checks. Raw stream assertions remain
available for framing-specific cases.

## Current Behavior Route

Use the current harness reference for implemented behavior:

- manifest string forms and physical multiline values;
- file-backed text operands such as `stdin_file` and `equals_file`;
- portable case-relative path validation and immutable text snapshots;
- authoritative discovery roots and generated/current inventory parity;
- encoded-line-break policy scanning in build preflight and generated-case
  runtime preflight;
- semantic baseline and checked migration evidence for existing cases; and
- structured JSON-RPC request fixtures, recursive `$case_text` expansion,
  envelope validation including duplicate standard member rejection, fixture
  diagnostics, and deterministic input framing.

This proposal does not redefine those contracts. Future JSON-RPC work must keep
those contracts intact and must add its implemented behavior to the harness
reference and executable evidence before this proposal closes.

## Remaining Goals

- Migrate representative LSP cases without weakening their previous assertion
  boundary.

## Non-Goals

- Changing Veln command output, LSP behavior, or JSON-RPC framing rules.
- Replacing all raw stdout checks with decoded assertions.
- Validating method-specific LSP parameter schemas in the harness.
- Replacing the implemented manifest grammar or sidecar model.
- Adding an allowlist for encoded line breaks in `case.toml`.

## Decoded LSP Assertions

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

Exactly one of `id` and `method` is required. `occurrence` is zero-based and
is valid only with `method`; it defaults to zero. Exactly one operation is
required: `equals`, `equals_file`, `contains`, or `missing = true`.
`equals_file` compares a selected JSON string with the exact file contents.
`contains` requires a selected JSON string containing the configured
substring.

Missing messages, duplicate response identifiers, invalid paths, wrong JSON
value kinds, and comparison failures identify the selector and assertion path.
Path absence can satisfy `missing = true` only after the enclosing response or
notification has been selected.

## Representation Choice

Authors use the smallest readable representation that preserves the assertion
intent.

| Content | Representation |
| --- | --- |
| Short text whose line structure belongs beside the assertion | Implemented multiline manifest string |
| Large exact text, reusable text, or native source text | Implemented case-relative text file |
| JSON-RPC requests sent to `veln lsp` | Implemented structured JSON-RPC request fixture |
| JSON-RPC response fields or notifications | Implemented decoded LSP assertion |
| Invalid or presentation-sensitive JSON-RPC framing | Existing raw stream assertion |

The harness must not choose representation from a size threshold. The manifest
author chooses the representation, subject to the completion gate.

## Migration Boundary

Existing raw LSP cases keep their current assertion meaning until they are
reviewed and migrated. A raw-to-structured input migration preserves the
ordered decoded request messages and uses the deterministic framing contract
above. A raw-output migration preserves the behavior-level assertion by
selecting the decoded message and path that were previously checked through
raw stream text.

Migration must not replace stable semantic checks with broad snapshots only to
move text out of a manifest. Framing-specific cases keep raw stdout or stderr
assertions.

## Implemented Decoded Assertion Evidence

The harness target now checks the decoded assertion foundation through the
following executable evidence. The harness reference is the current contract.

| Case | Expected result | Executable evidence |
| --- | --- | --- |
| Decode valid stdout containing responses and notifications. | Selectors find the requested response `id` or notification occurrence in output order. | `decoded_lsp_stream_selectors_and_json_pointer_object_matrix_succeed`. |
| Encounter duplicate response identifiers, malformed frames, trailing bytes, or partial frames. | The stream failure prevents semantic evaluation for that invocation. | `decoded_lsp_transport_failure_matrix_rejects_invalid_complete_streams`. |
| Select object keys and arrays with JSON Pointers, including escape and index boundaries. | Exact decoded keys and canonical in-range array indexes resolve; invalid traversal and absent values remain distinct. | The `decoded_lsp_*pointer*` tests. |
| Use every operation with existing, missing, and wrong-kind values or selectors. | Only the operation's declared value and kind contract succeeds. | The `decoded_lsp_operations_*` and `decoded_lsp_equals_file_*` tests. |
| Combine raw and semantic failures and run differing repeated streams. | Both consumers report independently, and repeat findings retain run and assertion order. | The `raw_stdout_and_decoded_lsp_*` and `repeated_run_failures_*` tests. |
| Record migrated LSP assertions in the semantic baseline. | The baseline captures each selector, path, operation, and operand so later migrations can be compared. | `examples/specification/lsp/publish-diagnostics/case.toml` and `toolchain-case-semantics.baseline`. |

## Remaining Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Migrate representative exact-output, fragment, formatting, repeated-run, and LSP cases. | Their observable checks match the pre-migration semantic baseline or an explicitly reviewed behavior-level conversion. | Focused migrated toolchain cases plus baseline comparison. |

## Verification Route

The existing harness target is the implementation route:

```sh
cargo test -p veln-cli --test toolchain_harness
```

Narrow filters are acceptable while developing one acceptance row, but proposal
completion requires the unfiltered harness target and the checked semantic
baseline or reviewed conversion records for migrated cases.

## Completion Gate

The proposal is complete only when all of the following conditions hold:

1. Representative LSP cases have been migrated without weakening their
   assertion boundary.
2. Raw stream assertions still cover framing-specific cases.
3. The implemented behavior is documented in
   [Toolchain Test Harness](../reference/toolchain-test-harness.md) and is
   backed by executable harness evidence.
4. The complete toolchain harness suite passes after migration.
5. This proposal is removed from `docs/proposals/`, and any durable completion
   evidence is moved to `docs/reference/implemented-proposals/` only if it
   remains useful.

## Remaining Implementation Boundary

- Migrate representative LSP cases from raw protocol streams to structured
  request fixtures and decoded assertions, with reviewed conversion evidence
  where raw bytes are not the behavior.
- Promote the remaining implemented behavior to the harness reference, keep
  executable evidence authoritative, and close this proposal route.
