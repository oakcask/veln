# Network Structured Transport Failures

Status: implemented

This record preserves the completed structured transport-failure target from
`../../proposals/network-effect-integration-boundary.md`. Current behavior is
specified by `../../specification/execution.md`,
`../../specification/names-effects.md`, and `../../specification/run-json.md`.

## Outcome

Public host listen, connect, accept, read, write, shutdown, stream-close, and
listener-close failures now share one runtime payload. It records a stable
operation and category, known local and peer endpoints, known listener or
stream identity, lifecycle phase, input/output/ownership commit facts, and
platform cause text. Unknown context is omitted rather than inferred.

Human output keeps the failed operation and stable category in the primary
line and renders context as notes. `run --json` projects the same facts under
`error.details` with `runtime.transport_failure` as the detail id. Transport
context stays at the runtime or adapter boundary and does not reclassify
schema, codec, HPACK, or HTTP/2 protocol failures.

## Evidence

- `../../../examples/specification/run/transport-socket-production-listen-failure-json/`
- `../../../examples/specification/run/transport-socket-connect-failure-json/`
- `../../../examples/specification/run/transport-socket-optional-accept-failure-json/`
- `../../../examples/specification/run/transport-socket-read-failure-json/`
- `../../../examples/specification/run/transport-socket-write-failure-human/`
- `../../../examples/specification/run/transport-socket-write-failure-json/`
- `../../../examples/specification/run/transport-socket-shutdown-partial-commit-json/`
- `../../../examples/specification/run/socket-stream-adapter-production-close-failure-json/`
- `../../../examples/specification/run/transport-socket-listener-close-failure-json/`
- `../../../examples/specification/run/transport-socket-stream-state-stale-write-json/`

The adjacent adapter failure cases preserve host-boundary classification, and
the existing protocol-core human and JSON cases continue to pin pure protocol
diagnostic precedence independently of transport failures.

## Read When

- Auditing why structured host failure work is no longer active proposal work.
- Checking completion evidence before changing transport failure projection.

## Skip Unless Needed

- Use the specification pages and executable cases for current behavior.
- Return to the proposal only for the remaining external socket target.
