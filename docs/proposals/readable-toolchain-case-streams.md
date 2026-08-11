---
role: proposal
update-when: The readable toolchain case stream proposal scope, structured JSON-RPC current-behavior route, decoded LSP assertion acceptance evidence, migration boundary, or completion gate changes.
---

# Readable Toolchain Case Streams

## Summary

The remaining proposal work is limited to decoded LSP response assertions,
representative LSP case migration, and proposal completion. Implemented
structured JSON-RPC request fixtures, manifest multiline syntax, case-relative
text sidecars, portable path checks, discovery inventory, encoded-line-break
policy, build preflight, runtime stale inventory barrier, and checked-in
case-text migration are current harness behavior. Their normative route is
[Toolchain Test Harness](../reference/toolchain-test-harness.md).

The remaining work lets LSP assertions target decoded response messages when
message structure is the observable behavior, then migrates representative
cases without weakening checks. Raw stream assertions remain available for
framing-specific cases.

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
  envelope validation, and deterministic input framing.

This proposal does not redefine those contracts. Future JSON-RPC work must keep
those contracts intact and must add its implemented behavior to the harness
reference and executable evidence before this proposal closes.

## Remaining Goals

- Assert LSP output by selecting decoded responses or notifications and then
  applying JSON path assertions.
- Preserve raw stream assertions for invalid framing, whitespace-sensitive
  framing, and other cases where bytes are the intended evidence.
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
| JSON-RPC response fields or notifications | Planned decoded LSP assertion |
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

## Acceptance Model

The rows below describe planned evidence only.

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Decode valid stdout containing responses and notifications. | Selectors find the requested response `id` or notification occurrence in output order. | Decoded stream selector cases. |
| Encounter duplicate response identifiers, malformed frames, trailing bytes, or partial frames. | Semantic LSP assertion preflight reports the stream failure without evaluating decoded assertions from an invalid stream. | Transport preflight failure matrix. |
| Select the message root, empty key, dotted key, numeric object key, slash or tilde key, space, and Unicode key with JSON Pointers. | Every token resolves by exact decoded scalar spelling, and the empty pointer selects the complete message. | JSON Pointer object-key matrix. |
| Decode `~0`, `~1`, `~01`, and adjacent escapes; reject invalid tilde escapes, nonempty paths without `/`, and URI fragments. | Valid escapes decode once in RFC order; invalid pointer syntax fails during manifest loading before resources or command execution. | Pointer syntax and escape-order table. |
| Apply array pointers with first, last, length, huge, leading-zero, signed, whitespace, non-ASCII, and `-` tokens. | Valid indexes select by position; invalid or out-of-range indexes report absent or invalid traversal without overflow. | Array traversal boundary matrix. |
| Use `missing = true` with an existing selected value and missing path, an existing path, and a missing LSP message. | Only the existing selection with a missing path succeeds. | Missing-path versus selector-failure cases. |
| Fail raw stdout while the decoded stream is valid, then fail a decoded assertion while raw stdout is valid. | Raw and decoded consumers remain independent; neither failure suppresses the other. | Raw and semantic independence truth table. |
| Run repeated invocations whose later runs differ in exit, framing, or selected values. | Each capturable invocation is checked independently and findings are grouped by run and manifest assertion order. | Scripted repeat aggregation and isolation case. |
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

1. Decoded LSP stream parsing, `[[lsp_assert]]`, and their failure behavior
   pass the planned evidence above.
2. Representative LSP cases have been migrated without weakening their
   assertion boundary.
3. Raw stream assertions still cover framing-specific cases.
4. The implemented behavior is documented in
   [Toolchain Test Harness](../reference/toolchain-test-harness.md) and is
   backed by executable harness evidence.
5. The complete toolchain harness suite passes after migration.
6. This proposal is removed from `docs/proposals/`, and any durable completion
   evidence is moved to `docs/reference/implemented-proposals/` only if it
   remains useful.

## Remaining Implementation Boundary

- Add decoded LSP stdout transport parsing, response and notification
  selection, JSON Pointer paths, and dependency-aware assertion aggregation.
- Migrate representative LSP cases from raw protocol streams to structured
  request fixtures and decoded assertions, with reviewed conversion evidence
  where raw bytes are not the behavior.
- Promote the remaining implemented behavior to the harness reference, keep
  executable evidence authoritative, and close this proposal route.
