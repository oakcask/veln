# HTTP/2 Standard Library Completion and Fixture Retirement

Status: proposed

This proposal defines the finite work required to retire the monolithic
`../../examples/specification/run/http2-protocol-core/` case. Current
implemented behavior belongs under `../specification/`; the implemented public
module boundary is summarized in `../specification/http2.md`.

## Problem

HTTP/2 is now an opt-in standard-library feature. Public frame, diagnostic,
HPACK, and stream-domain entry points exist under `std::http2`, and ordinary
projects load only the imported standard-module dependency closure.

Most protocol behavior and its strongest executable evidence still live in one
specification case instead of the standard package. That case currently owns:

- connection, stream-lifecycle, SETTINGS, flow-control, header-validation, and
  graceful-shutdown transitions;
- the production HPACK codec, dynamic-table state, header-field model, and
  fixture-specific compatibility paths;
- 65 `require_*` helper definitions used by 717 call sites;
- one complete stdout value and 315 output-chunk-list assertions; and
- local modules whose responsibilities overlap the new standard modules.

The fixture takes materially longer to analyze and execute than focused cases.
Deleting it now would remove coverage. Keeping it indefinitely would leave the
standard API incomplete and continue to make unrelated workspace verification
pay for a monolithic integration case.

## Target Boundary

The completed design has one implementation owner for each reusable behavior:

- `std::http2::frame` owns frame schema, decode, and encode behavior.
- `std::http2::diagnostic` owns protocol and peer-limit diagnostic conversion.
- `std::http2::hpack` owns the public HPACK codec and immutable state API.
- `std::http2::hpack::diagnostic` owns HPACK diagnostic conversion.
- `std::http2::core` owns sans-I/O connection and stream transitions.
- adjacent standard-library `*_test.veln` files own pure unit coverage.
- focused cases under `../../examples/specification/` own observable CLI,
  human-diagnostic, JSON, result-value, and output-chunk coverage.

The monolithic case is not an implementation module and is not a permanent
test utility. It may be deleted only after every behavior and observable
assertion has a classified replacement.

## Finite Remaining Scope

### HPACK Library Completion

Promote reusable behavior from `hpack_fixture.veln`, `hpack_static.veln`, and
`hpack_dynamic_core.veln` into responsibility-named private modules behind the
`std::http2::hpack` facade:

- Huffman encode and decode for arbitrary octets, including padding, EOS, and
  non-visible values;
- immutable dynamic-table insertion, eviction, capacity changes, index lookup,
  and size accounting;
- indexed, literal-with-indexing, literal-without-indexing, never-indexed, and
  table-size-update representations;
- recursive header lists with octet-preserving values; and
- immutable decode and encode transitions that expose no partial output or
  next state after failure.

Fixture display labels, canned header lists, stdout formatting, and expected
value construction do not belong in the standard package. Public HPACK names
must not retain the `hpack_fixture_` prefix.

The production codec must also replace the three remaining generic
`hpack.fixture.unsupported_header_block` fallback families with focused typed
failures:

1. indexed-field integer, zero-index, and unavailable-entry failures;
2. literal-field name, string-length, raw-octet, and Huffman failures; and
3. ordered-list encode failures after validation, integer encoding, string
   encoding, or active-capacity selection.

These families are bounded by the supported HPACK representations above. A new
fixture label or another same-shaped header-list example does not extend the
scope.

### Sans-I/O Core Completion

Promote the pure protocol state and transitions from `main.veln` and
`stream_domain.veln` into responsibility-named private modules behind
`std::http2::core`:

- connection preface and the initial peer SETTINGS gate;
- frame-kind, payload-length, stream-id-domain, and continuation sequencing;
- client, server, connection, promised, and peer-created stream domains;
- stream lifecycle, priority, reset, reservation, and monotonic peer stream
  admission;
- local and peer SETTINGS state, acknowledgement tracking, and value ranges;
- connection and per-stream receive and send flow-control windows;
- request, response, trailer, informational, no-content, CONNECT, and extended
  CONNECT header validation;
- content-length accounting;
- inbound HEADERS, CONTINUATION, DATA, PRIORITY, RST_STREAM, PUSH_PROMISE,
  SETTINGS, PING, WINDOW_UPDATE, and GOAWAY transitions;
- outbound DATA, HEADERS, PUSH_PROMISE, SETTINGS, PING, WINDOW_UPDATE,
  RST_STREAM, PRIORITY, and GOAWAY transitions; and
- graceful-shutdown admission and drain behavior.

Every transition is sans-I/O: it consumes explicit input and immutable state
and returns an action, next state, or typed failure. A rejected transition
preserves input state, HPACK state, flow-control credit, pending continuation,
and output bytes. Diagnostic precedence remains the same as the current
executable cases.

### Evidence Migration

Maintain a checked migration matrix while implementing this proposal. The
matrix has one row for every `require_*` call site, complete-stdout assertion,
and output-chunk-list assertion in the monolithic case. Each row records:

- the behavior or observable value it protects;
- its destination standard test or focused specification case;
- whether it covers success, failure, or failure-state preservation; and
- whether it protects a diagnostic id, human rendering, structured JSON,
  raw result value, CLI parsing, or emitted bytes.

Rows may be consolidated only when one parameterized or recursive test proves
the same invariant. Consolidation must name the shared invariant; it must not
discard a distinct endpoint role, starting state, transition, diagnostic
precedence rule, or output projection.

Pure cases move next to their owning standard module as small
`Result<(), String>` tests. Observable command behavior remains in focused
specification cases. The existing focused human and JSON cases should be
reused before creating new directories.

The complete stdout string is retained until all of its lines are classified.
It is removed as one change after the matrix reaches zero unclassified lines;
it is not gradually weakened with broader substring matching.

## Implementation Order

Each step must leave the standard package and workspace buildable:

1. Complete HPACK state and codec behavior, then move pure HPACK assertions to
   adjacent tests.
2. Complete connection, stream, settings, flow-control, headers, and shutdown
   transitions, moving pure assertions after each responsibility lands.
3. Move remaining human, JSON, CLI parsing, and output-chunk assertions to
   focused specification cases.
4. Audit old symbols, local fixture types, `require_*` calls, stdout lines, and
   output chunks. Classify every residual before deleting anything.
5. Delete the monolithic case and promote the final implemented behavior to
   `../specification/http2.md`. Archive this proposal under
   `../reference/implemented-proposals/` and remove it from the proposal
   catalog.

Do not combine a large implementation move with deletion of its only existing
coverage. The source fixture remains available until the replacement tests
pass independently.

## Verification Gates

### Standard Package

- Frame, HPACK, diagnostics, and core tests cover their responsibility without
  importing fixture modules.
- The toolchain-owned standard project checks all source files, including
  modules unreachable from the prelude.
- Private implementation modules cannot be imported from user packages.
- Bare former HTTP/2 and HPACK helpers remain unavailable.

### Protocol Semantics

- Frame boundaries, preface, SETTINGS, continuation, stream lifecycle,
  flow-control, GOAWAY, header validation, and every supported HPACK
  representation have focused success and failure coverage.
- Failed decode and send transitions prove state, input, and output
  preservation.
- Diagnostic ids, human rendering, and structured protocol details remain
  stable, while raw nested diagnostic values use the intended new ADT shape.

### Loader and Performance

- An ordinary project does not load or analyze HTTP/2 modules.
- Importing one HTTP/2 facade loads only its dependency closure.
- Record guarded-run elapsed time and peak memory for an ordinary project, one
  HTTP/2 import, all standard HTTP/2 tests, and the monolithic case before its
  deletion.
- Investigate timeouts, kills, allocation failures, or material regression as
  implementation defects rather than increasing the runner limit.

Use `scripts/agent-run` and `scripts/agent-test` for the broad, generated, and
workspace checks.

## Fixture Deletion Gate

The directory `../../examples/specification/run/http2-protocol-core/` is
deletable only when all of the following are true:

- the migration matrix has zero unclassified `require_*` calls, stdout lines,
  and output-chunk assertions;
- no current implementation, standard test, or focused specification case
  imports any file from the directory;
- no reusable HPACK, stream, connection, settings, flow-control, header, or
  shutdown implementation remains there;
- standard package tests and focused specification cases pass without the
  directory;
- the workspace test suite passes after the directory is removed;
- old public symbols and fixture-only public codec names have no unclassified
  residuals; and
- `../specification/http2.md` describes the resulting implemented API and
  routes to its executable evidence.

When the gate is met, remove `main.veln`, `stream_domain.veln`,
`hpack_fixture.veln`, `hpack_static.veln`, `hpack_dynamic_core.veln`, and the
monolithic `case.toml` together. A smaller example may keep the directory name
only if it has a focused observable purpose and no longer contains the broad
fixture implementation or complete stdout assertion.

## Non-Goals

- TLS, ALPN, socket listeners, or platform networking
- mutable or effectful connection ownership inside the pure core
- production throughput optimization unrelated to removing an observed
  regression
- compatibility aliases for the former bare HTTP/2 or `hpack_fixture_*` API
- weakening diagnostics or JSON assertions to make migration easier
- adding another fixture case solely to increase a count, list width, table
  update count, or stream sequence length

## Completion Criteria

- The fixture deletion gate is satisfied and the monolithic case is removed or
  reduced to a focused example.
- `std::http2::hpack` and `std::http2::core` own all reusable behavior formerly
  implemented by the fixture.
- The three generic production HPACK fallback families return focused typed
  failures.
- Standard tests and focused specification cases preserve all classified
  semantics and observable output.
- Current specification and executable evidence are updated before this
  proposal is archived as implemented history.
