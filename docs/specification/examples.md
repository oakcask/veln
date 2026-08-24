---
role: routing
update-when: The executable specification case directory layout, fixture evidence placement, case grouping, or example routing guidance changes.
---

# Examples

Executable examples live under `../../examples/specification/`. Use that
directory's README and the focused `case.toml` files as the source of checked
behavior.

Identifier casing and quarantined recovery are checked by
[`identifier-casing-json`](../../examples/specification/check/identifier-casing-json/case.toml),
[`identifier-casing-human`](../../examples/specification/check/identifier-casing-human/case.toml),
[`identifier-casing-quarantine`](../../examples/specification/check/identifier-casing-quarantine/case.toml), and
[`identifier-casing-value-recovery`](../../examples/specification/check/identifier-casing-value-recovery/case.toml).
Public alias declaration-name casing is checked by
[`identifier-casing-public-alias-declarations`](../../examples/specification/check/identifier-casing-public-alias-declarations/case.toml).
Test declaration-name casing without ordinary function-call recovery is
checked by
[`identifier-casing-test-declaration`](../../examples/specification/check/identifier-casing-test-declaration/case.toml).
Duplicate overlap and alias-mismatch suppression boundaries are checked by
[`identifier-casing-duplicate-quarantine`](../../examples/specification/check/identifier-casing-duplicate-quarantine/case.toml),
[`identifier-casing-alias-mismatch`](../../examples/specification/check/identifier-casing-alias-mismatch/case.toml), and
[`identifier-casing-alias-same-leaf-mismatch`](../../examples/specification/check/identifier-casing-alias-same-leaf-mismatch/case.toml).
Same-file valid declaration precedence over a quarantined alias is checked by
[`identifier-casing-same-file-alias-valid-wins`](../../examples/specification/check/identifier-casing-same-file-alias-valid-wins/case.toml).
Invalid public function aliases preserve independent target and same-file use
diagnostics in
[`identifier-casing-invalid-alias-missing-target`](../../examples/specification/check/identifier-casing-invalid-alias-missing-target/case.toml),
[`identifier-casing-invalid-alias-wrong-kind`](../../examples/specification/check/identifier-casing-invalid-alias-wrong-kind/case.toml),
[`identifier-casing-invalid-alias-same-file-use`](../../examples/specification/check/identifier-casing-invalid-alias-same-file-use/case.toml), and
[`identifier-casing-invalid-alias-duplicates`](../../examples/specification/check/identifier-casing-invalid-alias-duplicates/case.toml).
Invalid type alias quarantine preserves independently provable type
mismatches in
[`identifier-casing-type-alias-quarantine`](../../examples/specification/check/identifier-casing-type-alias-quarantine/case.toml).
Invalid source ADT type declarations are quarantined from schema/type lookup
and binary schema payload wrong-kind checks in
[`identifier-casing-schema-type-quarantine`](../../examples/specification/check/identifier-casing-schema-type-quarantine/case.toml).
Split recovery candidate uniqueness is checked by
[`identifier-casing-split-recovery-candidates`](../../examples/specification/check/identifier-casing-split-recovery-candidates/case.toml).
Handler callable and pattern-binding recovery boundaries are checked by
[`identifier-casing-handler-callable-recovery`](../../examples/specification/check/identifier-casing-handler-callable-recovery/case.toml)
and
[`identifier-casing-pattern-binding-recovery`](../../examples/specification/check/identifier-casing-pattern-binding-recovery/case.toml).
Inferred callable `let` and pattern-binding recovery is checked by
[`identifier-casing-inferred-callable-recovery`](../../examples/specification/check/identifier-casing-inferred-callable-recovery/case.toml).
The selected-entry command boundary is checked by
[`identifier-casing-reachable`](../../examples/specification/run/identifier-casing-reachable/case.toml)
and
[`identifier-casing-unreachable`](../../examples/specification/run/identifier-casing-unreachable/case.toml).
Invalid casing on the selected entry declaration itself is checked by
[`identifier-casing-invalid-entry`](../../examples/specification/run/identifier-casing-invalid-entry/case.toml).
Import and public-alias quarantine are checked by
[`identifier-casing-import-quarantine`](../../examples/specification/run/identifier-casing-import-quarantine/case.toml)
and
[`identifier-casing-alias-quarantine`](../../examples/specification/run/identifier-casing-alias-quarantine/case.toml).
Imported invalid public-alias quarantine is checked by
[`identifier-casing-imported-invalid-alias-quarantine`](../../examples/specification/run/identifier-casing-imported-invalid-alias-quarantine/case.toml).
Reachable and unreachable public alias declaration-name boundaries are checked
by
[`identifier-casing-reachable-function-alias`](../../examples/specification/run/identifier-casing-reachable-function-alias/case.toml),
[`identifier-casing-reachable-type-alias`](../../examples/specification/run/identifier-casing-reachable-type-alias/case.toml), and
[`identifier-casing-unreachable-public-alias`](../../examples/specification/run/identifier-casing-unreachable-public-alias/case.toml).
Mixed reachable `run --json` casing and non-casing diagnostics are checked by
[`identifier-casing-mixed-json-diagnostics`](../../examples/specification/run/identifier-casing-mixed-json-diagnostics/case.toml).
Resolution-aware run reachability is checked by
[`identifier-casing-valid-function-vs-invalid-constructor`](../../examples/specification/run/identifier-casing-valid-function-vs-invalid-constructor/case.toml),
[`identifier-casing-valid-constructor-vs-invalid-function`](../../examples/specification/run/identifier-casing-valid-constructor-vs-invalid-function/case.toml),
[`identifier-casing-imported-constructor-valid-wins`](../../examples/specification/run/identifier-casing-imported-constructor-valid-wins/case.toml),
[`identifier-casing-handler-imported-constructor-valid-wins`](../../examples/specification/run/identifier-casing-handler-imported-constructor-valid-wins/case.toml),
[`identifier-casing-qualified-same-leaf`](../../examples/specification/run/identifier-casing-qualified-same-leaf/case.toml), and
[`identifier-casing-alias-transitive-target`](../../examples/specification/run/identifier-casing-alias-transitive-target/case.toml).
Resolved local bindings and ADT payload closure are checked by
[`identifier-casing-local-binding-vs-invalid-constructor`](../../examples/specification/run/identifier-casing-local-binding-vs-invalid-constructor/case.toml)
and
[`identifier-casing-adt-payload-closure`](../../examples/specification/run/identifier-casing-adt-payload-closure/case.toml).
Handler and underscore-led type reachability boundaries are checked by
[`identifier-casing-unused-handler-type-reference`](../../examples/specification/run/identifier-casing-unused-handler-type-reference/case.toml),
[`identifier-casing-transitive-handler-binding`](../../examples/specification/run/identifier-casing-transitive-handler-binding/case.toml), and
[`identifier-casing-underscore-type-closure`](../../examples/specification/run/identifier-casing-underscore-type-closure/case.toml).
Run diagnostics outside the casing filter remain checked by
[`unreachable-duplicate-constructor-diagnostic`](../../examples/specification/run/unreachable-duplicate-constructor-diagnostic/case.toml),
[`unreachable-type-alias-diagnostic`](../../examples/specification/run/unreachable-type-alias-diagnostic/case.toml), and
[`unreachable-handler-diagnostic`](../../examples/specification/run/unreachable-handler-diagnostic/case.toml).

Case text files under `case-text/` are fixture evidence owned by the CLI
toolchain harness. Their placement can change how an example is reviewed
without changing the language or command behavior that the case checks.
Text sidecars can also hold parsed JSON operands for `equals_json_file`
assertions when that keeps large or nested expected values reviewable.
Structured JSON-RPC request fixtures can use `$workspace_file_uri` directives
when an example needs the canonical URI for a copied workspace file without
recording a temporary path in the fixture.
Files ending in `.raw` are exact-byte fixture sidecars. Use them when checkout
line-ending normalization would change the protocol bytes that the example
feeds to the CLI, including LSP JSON-RPC stdin streams.

## Inference Routes

- Local binding and initializer inference:
  `../../examples/specification/check/local-let-inference/` and
  `../../examples/specification/check/local-let-inference-diagnostics/`.
- Constructor and pattern context:
  `../../examples/specification/check/adt-constructor-inference/` and
  `../../examples/specification/check/match-scrutinee-inference/`.
- Callback expected types:
  `../../examples/specification/check/prelude-callback-argument-inference/`,
  `../../examples/specification/check/declared-helper-callback-inference/`,
  `../../examples/specification/check/callback-return-expected-type-inference/`,
  and `../../examples/specification/check/collection-callback-inference/`.

## Binary Schema Routes

- Integer bitwise semantics and chained and mixed runtime-checked contracts:
  `../../examples/specification/run/integer-bitwise-operators/`.
- Literal and dynamic invalid shift diagnostics:
  `../../examples/specification/check/integer-bitwise-invalid-literal-shifts/`,
  `../../examples/specification/check/integer-bitwise-invalid-literal-shifts-human/`,
  `../../examples/specification/run/integer-bitwise-invalid-dynamic-shift-human/`,
  and
  `../../examples/specification/run/integer-bitwise-invalid-dynamic-shift-json/`.
- Exact-width unsigned replacement shapes and both-byte-order bit operations:
  `../../examples/specification/check/binary-schema-uint-replacement-shapes/`
  and
  `../../examples/specification/run/binary-schema-uint-bit-operations-both-byte-orders/`.
- Schema-local helper projection:
  `../../examples/specification/run/binary-schema-local-projection-boundary/`.
- Parser rejection for schema-level `map to`:
  `../../examples/specification/check/schema-map-to-rejected/`,
  `../../examples/specification/check/schema-map-to-selector-rejected/`, and
  `../../examples/specification/check/schema-map-to-inverse-rejected/`.
- Same-module recursive dispatch helper decode and primitive-base encode:
  `../../examples/specification/run/binary-schema-recursive-dispatch-decode-encode/`.
- Same-module recursive dispatch missing primitive base rejection:
  `../../examples/specification/run/binary-schema-recursive-dispatch-rejected/`.

## Network Routes

- HTTP/2 duplex-stream connection driver behavior:
  `../../examples/specification/run/http2-connection-server-split-preface/`,
  `../../examples/specification/run/http2-connection-settings-ack/`,
  `../../examples/specification/run/http2-connection-partial-frame/`,
  `../../examples/specification/run/http2-connection-clean-end/`,
  `../../examples/specification/run/http2-connection-truncated-end-json/`,
  `../../examples/specification/run/http2-connection-protocol-failure-json/`,
  `../../examples/specification/run/http2-connection-closed-entry/`,
  `../../examples/specification/run/http2-connection-client-initial-output/`,
  and
  `../../examples/specification/run/http2-connection-tcp-loopback-client/`.
- HTTP/2 server application boundary behavior:
  `../../examples/specification/check/http2-connection-application-boundary-effects/`,
  `../../examples/specification/run/http2-connection-application-one-request/`,
  `../../examples/specification/run/http2-connection-application-callback-failure-json/`,
  `../../examples/specification/run/http2-connection-application-unsupported-request-json/`,
  `../../examples/specification/run/http2-connection-application-second-request-json/`,
  `../../examples/specification/run/http2-connection-application-invalid-actions-json/`,
  and
  `../../examples/specification/run/http2-connection-application-rejected-action-json/`.
- HTTP/2 server service boundary behavior:
  `../../examples/specification/check/http2-service-transport-effect-replacement/`,
  `../../examples/specification/check/http2-service-task-effect-row/`,
  `../../examples/specification/check/http2-service-task-handler-boundary/`,
  `../../examples/specification/run/http2-service-two-connections/`,
  `../../examples/specification/run/http2-service-callback-failure/`,
  `../../examples/specification/run/http2-service-join-failure-json/`,
  `../../examples/specification/run/http2-service-protocol-failure-json/`,
  and
  `../../examples/specification/run/http2-service-transport-failure-json/`.
- HTTP/2 client service boundary behavior:
  `../../examples/specification/check/http2-client-service-effect-row/`,
  `../../examples/specification/run/http2-client-service-reuse-boundary/`,
  and
  `../../examples/specification/run/http2-client-service-callback-failure/`.
- Fixture-backed listener endpoint text, accepted-stream endpoint text, read,
  stream close, and listener close:
  `../../examples/specification/run/transport-socket-listener-address/`.
- Production-loopback listener endpoint text, accepted-stream endpoint text,
  read, stream close, and listener close:
  `../../examples/specification/run/transport-socket-production-listener-address/`.
- Production-loopback read-side shutdown lifecycle:
  `../../examples/specification/run/socket-stream-adapter-production-shutdown-read-lifecycle/`.
- `net::shutdown_read` effect checking:
  `../../examples/specification/check/socket-stream-shutdown-read-effects/`.
- Production-loopback write-side shutdown lifecycle:
  `../../examples/specification/run/socket-stream-adapter-production-shutdown-write-lifecycle/`.
- Production-loopback stream state inspection:
  `../../examples/specification/run/transport-socket-stream-state-inspection/`.
- Production-loopback stale stream handle failure after state inspection:
  `../../examples/specification/run/transport-socket-stream-state-stale-write-json/`.
- `net::stream_can_read`, `net::stream_can_write`, and
  `net::stream_is_closed` effect checking:
  `../../examples/specification/check/transport-socket-stream-state-effects/`.
- Fixture-backed write-side shutdown lifecycle and failure:
  `../../examples/specification/run/socket-stream-adapter-shutdown-write-lifecycle/`
  and
  `../../examples/specification/run/socket-stream-adapter-shutdown-write-failure-json/`.
- `net::shutdown_write` effect checking:
  `../../examples/specification/check/socket-stream-shutdown-write-effects/`.
- Listener endpoint metadata lookup runtime failure JSON:
  `../../examples/specification/run/transport-socket-listener-address-failure-json/`.
- `net::listener_local_addr` effect checking:
  `../../examples/specification/check/transport-socket-listener-address-effects/`.
- Cancellable adapter write-drain helper completion, deadline, cancellation,
  and effect checking:
  `../../examples/specification/run/socket-stream-adapter-cancellable-write-drain/`,
  `../../examples/specification/run/socket-stream-adapter-cancellable-write-drain-deadline/`,
  `../../examples/specification/run/socket-stream-adapter-cancellable-write-drain-cancelled/`,
  and
  `../../examples/specification/check/socket-stream-adapter-cancellable-write-drain-effects/`.
- Adapter accept-loop helper success, forced transport failures, and effect
  checking:
  `../../examples/specification/run/socket-stream-adapter-accept-loop/`,
  `../../examples/specification/run/socket-stream-adapter-accept-loop-accept-failure-json/`,
  `../../examples/specification/run/socket-stream-adapter-accept-loop-read-failure-json/`,
  `../../examples/specification/run/socket-stream-adapter-accept-loop-write-failure-json/`,
  `../../examples/specification/run/socket-stream-adapter-accept-loop-stream-close-failure-json/`,
  `../../examples/specification/run/socket-stream-adapter-accept-loop-listener-close-failure-json/`,
  and
  `../../examples/specification/check/socket-stream-adapter-accept-loop-effects/`.

## Diagnostic Routes

- Schema field references and helper eligibility:
  `../../examples/specification/check/binary-schema-field-reference-diagnostics/`
  and
  `../../examples/specification/check/binary-schema-dispatch-payload-helper-boundary-json/`.
- Codec and schema byte failures: the focused `codec-*` and `binary-schema-*`
  cases under `../../examples/specification/run/`.
- Typed protocol failures and projection: the focused `http2-*` cases under
  `../../examples/specification/run/`.

## Read When

- Updating executable specification coverage.
- Checking which public CLI behavior is pinned by a case.

## Agent Protocol Routes

- MCP stdio lifecycle, workspace-project inventory, tool schemas, refresh,
  initialization phase boundaries, request metadata, and JSON-RPC framing:
  `../../examples/specification/mcp/workspace-lifecycle/`.
- MCP saved project diagnostics, `check_project` tool schema advertising, and
  spanless related-note projection:
  `../../examples/specification/mcp/check-project-diagnostics/`.
- MCP anonymous single-file `check_project` isolation from other saved
  workspace sources:
  `../../examples/specification/mcp/anonymous-single-file-isolation/`.
- MCP saved workspace `definition` tool schema advertising, declaration
  location, no-definition, numeric coordinate spellings, invalid-position and
  schema-rejection results, and response-local assertion coverage:
  `../../examples/specification/mcp/definition-workspace/`.
