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

- Source identifier casing diagnostics, accepted-name absence checks,
  valid-symbol precedence, import-path and source-path module casing, and
  ambiguous recovery refusal:
  `../../examples/specification/check/identifier-casing-source-recovery-json/`,
  `../../examples/specification/check/identifier-casing-binding-positions-json/`,
  `../../examples/specification/check/identifier-casing-owned-constructor-recovery-json/`,
  `../../examples/specification/check/identifier-casing-owned-constructor-recovery-human/`,
  `../../examples/specification/check/identifier-casing-function-value-recovery-json/`,
  `../../examples/specification/check/identifier-casing-function-value-recovery-human/`,
  `../../examples/specification/check/identifier-casing-underscore-recovery-json/`,
  `../../examples/specification/check/identifier-casing-import-recovery-isolation-json/`,
  `../../examples/specification/check/identifier-casing-public-alias-recovery-isolation-json/`,
  `../../examples/specification/check/identifier-casing-public-alias-targets-json/`,
  `../../examples/specification/check/identifier-casing-public-alias-targets-human/`,
  `../../examples/specification/check/identifier-casing-accepted-names-json/`,
  `../../examples/specification/check/identifier-casing-valid-symbol-precedence-json/`,
  `../../examples/specification/check/identifier-casing-implicit-prelude-boundary-json/`,
  `../../examples/specification/check/identifier-casing-implicit-prelude-isolation-json/`,
  `../../examples/specification/check/identifier-casing-handler-binding-quarantine-json/`,
  `../../examples/specification/check/identifier-casing-cross-class-ambiguous-recovery-json/`,
  `../../examples/specification/check/identifier-casing-qualified-constructor-pattern-json/`,
  `../../examples/specification/check/identifier-casing-qualified-constructor-pattern-human/`,
  `../../examples/specification/check/identifier-casing-qualified-constructor-pattern-over-suppression-json/`,
  `../../examples/specification/check/identifier-casing-qualified-constructor-pattern-direct-diagnostics-json/`,
  `../../examples/specification/check/identifier-casing-qualified-constructor-pattern-type-mismatch-json/`,
  `../../examples/specification/check/identifier-casing-qualified-use-paths-json/`,
  `../../examples/specification/check/identifier-casing-qualified-use-paths-human/`,
  `../../examples/specification/check/identifier-casing-declaration-type-carriers-json/`,
  `../../examples/specification/check/identifier-casing-declaration-type-carriers-human/`,
  `../../examples/specification/check/identifier-casing-qualified-use-recovery-controls-json/`,
  `../../examples/specification/check/identifier-casing-qualified-use-recovery-controls-human/`,
  `../../examples/specification/check/identifier-casing-qualified-handler-boundaries-json/`,
  `../../examples/specification/check/identifier-casing-qualified-handler-boundaries-human/`,
  `../../examples/specification/check/identifier-casing-module-header-json/`,
  `../../examples/specification/check/identifier-casing-module-header-accepted-json/`,
  `../../examples/specification/check/identifier-casing-source-path-json/`,
  `../../examples/specification/check/identifier-casing-exported-source-path-json/`,
  `../../examples/specification/check/identifier-casing-source-path-human/`,
  `../../examples/specification/check/identifier-casing-chained-companion-boundary-json/`,
  `../../examples/specification/check/identifier-casing-import-path-json/`,
  `../../examples/specification/check/identifier-casing-import-path-human/`,
  `../../examples/specification/check/identifier-casing-import-missing-module-overlap-json/`,
  `../../examples/specification/check/identifier-casing-import-duplicate-overlap-json/`,
  `../../examples/specification/check/identifier-casing-import-alias-cascade-boundary-json/`,
  `../../examples/specification/check/identifier-casing-import-type-cascade-boundary-json/`,
  `../../examples/specification/check/identifier-casing-import-constructor-cascade-boundary-json/`,
  `../../examples/specification/check/identifier-casing-import-effect-cascade-boundary-json/`,
  `../../examples/specification/check/identifier-casing-import-handler-cascade-boundary-json/`,
  `../../examples/specification/check/identifier-casing-import-order-json/`,
  `../../examples/specification/check/identifier-casing-import-missing-type-control-json/`,
  `../../examples/specification/check/identifier-casing-import-missing-type-export-json/`,
  `../../examples/specification/check/identifier-casing-import-missing-constructor-control-json/`,
  `../../examples/specification/check/identifier-casing-import-schema-cascade-boundary-json/`,
  `../../examples/specification/check/identifier-casing-import-private-schema-boundary-json/`,
  `../../examples/specification/check/identifier-casing-namespace-use-roles/`,
  and
  `../../examples/specification/check/identifier-casing-ambiguous-recovery-json/`.
- Source identifier casing run reachability, recovery, import isolation,
  qualified type path diagnostics, same-owner constructor ambiguity, handler
  annotations, handler clause expressions, and non-name record fields:
  `../../examples/specification/run/identifier-casing-reachable-recovery-json/`,
  `../../examples/specification/run/identifier-casing-constructor-call-recovery-json/`,
  `../../examples/specification/run/identifier-casing-reachable-invalid-alias-json/`,
  `../../examples/specification/run/identifier-casing-reachable-expression-type-json/`,
  `../../examples/specification/run/identifier-casing-reachable-type-alias-json/`,
  `../../examples/specification/run/identifier-casing-unreachable-peer/`,
  `../../examples/specification/run/identifier-casing-owned-nullary-constructor-recovery-json/`,
  `../../examples/specification/run/identifier-casing-owned-payload-constructor-recovery-json/`,
  `../../examples/specification/run/identifier-casing-function-value-recovery-json/`,
  `../../examples/specification/run/identifier-casing-import-alias-run-boundary-json/`,
  `../../examples/specification/run/identifier-casing-import-recovery-isolation-json/`,
  `../../examples/specification/run/identifier-casing-qualified-type-import-isolation-json/`,
  `../../examples/specification/run/identifier-casing-valid-function-value-precedence-json/`,
  `../../examples/specification/run/identifier-casing-cross-class-ambiguous-recovery-json/`,
  `../../examples/specification/run/identifier-casing-owned-constructor-ambiguous-recovery-json/`,
  `../../examples/specification/run/identifier-casing-owned-constructor-ambiguous-recovery-human/`,
  `../../examples/specification/run/identifier-casing-same-name-recovery-arity-json/`,
  `../../examples/specification/run/identifier-casing-reachable-handler-annotation-json/`,
  `../../examples/specification/run/identifier-casing-reachable-handler-bindings-json/`,
  `../../examples/specification/run/identifier-casing-reachable-handler-clauses-json/`,
  `../../examples/specification/run/identifier-casing-loaded-dependency-json/`,
  `../../examples/specification/run/identifier-casing-loaded-unreachable-dependency-json/`,
  `../../examples/specification/run/identifier-casing-unloaded-dependency-json/`,
  `../../examples/specification/run/identifier-casing-unselected-import-path-json/`,
  `../../examples/specification/run/identifier-casing-unused-import-path-json/`,
  `../../examples/specification/run/identifier-casing-module-header-json/`,
  and `../../examples/specification/run/identifier-casing-record-field-reachability/`.
- Source identifier casing selected-suite static gates, unselected test peer
  isolation, exact companion recovery isolation, selected documentation-source
  diagnostics, and excluded documentation-source or companion isolation, plus
  LSP workspace selection, invalid-symbol recovery navigation including a
  callable parameter call target, class-preserving rename validation, and
  rename conflict rejection:
  `../../examples/specification/test/identifier-casing-selected-static-gate-json/`,
  `../../examples/specification/test/identifier-casing-companion-target-recovery-isolation-json/`,
  `../../examples/specification/test/identifier-casing-companion-source-recovery-isolation-json/`,
  `../../examples/specification/test/identifier-casing-companion-target-binding-recovery-isolation-json/`,
  `../../examples/specification/test/identifier-casing-companion-source-binding-recovery-isolation-json/`,
  `../../examples/specification/test/identifier-casing-unselected-peer-json/`,
  `../../examples/specification/doc/identifier-casing-included-source/`,
  `../../examples/specification/doc/identifier-casing-excluded-source/`,
  `../../examples/specification/doc/identifier-casing-excluded-companion/`,
  `../../examples/specification/lsp/identifier-casing-snapshot-boundary/`,
  `../../examples/specification/lsp/identifier-casing-overlay-boundary/`,
  `../../examples/specification/lsp/identifier-casing-recovery-navigation/`,
  `../../examples/specification/lsp/identifier-casing-source-path-boundary/`,
  `../../examples/specification/lsp/identifier-casing-handler-binding-navigation/`,
  `../../examples/specification/lsp/identifier-casing-qualified-use-navigation/`,
  `../../examples/specification/lsp/identifier-casing-qualified-module-type-navigation/`,
  `../../examples/specification/lsp/identifier-casing-qualified-prelude-navigation/`,
  `../../examples/specification/lsp/identifier-casing-qualified-function-navigation/`,
  `../../examples/specification/lsp/identifier-casing-qualified-import-alias-navigation/`,
  and `../../examples/specification/lsp/identifier-casing-rename-boundary/`.
  The rename boundary case includes type alias conflict rejection in the
  current type namespace, same-clause handler operation parameter conflict
  rejection, function-to-test duplicate rejection, imported function ambiguity
  rejection for call and function-value occurrences, constructor ambiguity
  rejection through public type-alias re-export visibility, effect operation
  role exclusion from constructor rename visibility and edits, and
  declaration-location reporting for parameter, result-binding, and handler
  parameter lexical conflicts.
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
- MCP saved workspace `definition` recovery navigation for unique invalid
  source declarations, ambiguous recovery refusal, and valid-symbol
  precedence:
  `../../examples/specification/mcp/definition-recovery-navigation/`.
