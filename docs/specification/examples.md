---
role: routing
update-when: The executable specification case directory layout, fixture evidence placement, case grouping, or example routing guidance changes.
---

# Examples

Executable examples live under `../../examples/specification/`. Use that
directory's README and the focused `case.toml` files as the source of checked
behavior.

Case text files under `case-text/` are fixture evidence owned by the CLI
toolchain harness. Their placement can change how an example is reviewed
without changing the language or command behavior that the case checks.
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
  location, canonical workspace-file URI spelling, no-definition,
  invalid-position results, and numeric coordinate schema rejection:
  `../../examples/specification/mcp/definition-workspace/`.
