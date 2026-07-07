# Examples

Status: routing

Executable examples live under `../../examples/specification/`. Use that
directory's README and the focused `case.toml` files as the source of checked
behavior.

## Binary Schema Routes

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

## Read When

- Updating executable specification coverage.
- Checking which public CLI behavior is pinned by a case.
