---
role: specification
authority: normative
update-when: The Veln effect label contract, compiler-known effectful call surface, or executable effects evidence changes.
---

# Effects

This page specifies effect labels and compiler-known effectful calls.

## Effect Labels

Implemented effect labels are:

- `stdio`
- `fs`
- `net`
- `db`
- `time`
- `random`
- `process`
- `concurrency`

Function and test `effects [...]` declarations may name these labels. A
declaration that names any other effect reports `effect.unknown` at the
unknown effect label. The checker currently infers `stdio`, `fs`,
`net`, `time`, `process`, and `concurrency` from compiler-known calls. The
other labels are reserved coarse-grained public boundary labels for source
compatibility.

One user-defined function declaration may bind one effect row with
`<effect E>`. The bound row may appear as the final `...E` entry in that
function's declared effect set and in nested function type effect sets inside
the function signature. An unbound row tail, more than one row tail in one
effect set, or a row tail before a later effect entry is rejected. When a
row-polymorphic function is called with a callback argument, the callback's
duplicate-free concrete effect set is substituted for `E` and unioned with the
concrete effects written beside `...E`. Public boundary diagnostics for the
call use the concrete instantiated effects. The checked
`effect-row-syntax-diagnostics` and `http2-service-effect-row` specification
cases fix the syntax failures, empty substitution, non-empty substitution,
duplicate removal, callback compatibility, and concrete handler replacement.

Source modules may also declare nominal operation effects with `effect Name`
or `pub effect Name`. Each operation declares ordinary parameter types and one
result type. The effect name is owned by its module. Same-module declarations
may use the short name in `effects [...]` lists and `perform` expressions.
Imported declarations use the written module path, such as
`effects [transport::DuplexStream]` and
`perform transport::DuplexStream::read_chunk()`. Function type annotations use
the same effect-list spelling.
Unknown nominal effects in function type annotations are rejected at the
effect path inside the annotation.

`perform E::operation(arguments)` resolves `E` as a nominal effect and resolves
`operation` in that effect declaration. The checker validates the argument
types against the declared operation parameter types. The expression type is
the declared operation result type. The expression contributes the nominal
effect to the containing function's inferred effect set. Public functions must
declare that effect. Private functions use the existing private effect
inference rule. Duplicate operation declarations are reported at the duplicate
operation name span and include the first declaration as related context. An
unknown operation is reported at the operation name span. An unknown performed
effect is reported at the performed effect path. Runnable `veln run` and
`veln test` entry boundaries reject a retained user-defined effect before JVM
execution, including effects inferred for a private run entry. Exported
library functions may retain user-defined effects in their signatures. The
checked behavior is specified by
`examples/specification/check/user-effect-operation-boundaries/`,
`examples/specification/run/user-effect-runnable-boundary/`, and
`examples/specification/test/user-effect-test-boundary/`.
When a `.test.veln` companion writes an explicit `use` for its exact target,
the same qualified target path can name a private target nominal effect in
`perform`, declaration effect lists, function type annotation effect lists,
companion-local handler `handles` clauses, and declared handler effect lists.
The permission is exact and
non-transitive. Bare names, missing imports, wrong-target companions,
`_test.veln` integration modules, and external packages do not receive this
private effect access. The checked cases are routed from
`source-surface.md`.

Lexical handlers provide all operations of one nominal effect for the dynamic
evaluation of one `handle Body with Handler(arguments)` expression. A handler
declaration names context parameters, the handled effect, an optional
`effects [...]` list, and one operation clause per handled operation. An
operation clause binds the operation arguments and evaluates an ordinary
expression. Handler context parameters are in lexical scope for each clause.
A handler declaration is rejected when an operation clause is missing,
repeated, names an operation absent from the handled effect, binds the wrong
number of operation parameters, or repeats a clause binding. A clause body
must return the operation result type and must not retain the handled effect.
A public handler must declare every other effect retained by its clauses. A
private handler infers retained effects from its clauses. Declared handler
effect lists are canonical, unordered, and duplicate-free.

The checker evaluates the effect set of a handle expression as the union of
context argument effects, body effects with the handled nominal effect removed,
and the handler declaration effects. Handler context arguments are evaluated
left to right before the body. Handling is deep for calls made during the body,
and nested handlers for the same operation shadow outer handlers until the
nested body finishes. Handler state is lexical to the current task. Task
creation expressions expose their job effect rows at the call expression, so a
lexical handler around the task creation expression can discharge a handled
nominal job effect before the runnable entry boundary is checked. The checked
behavior is specified by the lexical-handler and handler-operation cases under
`examples/specification/`, including the task-boundary, early-return cleanup,
public handler effect-declaration, and `veln test` success cases.

The exported standard `transport` module declares this public nominal effect:

```veln
pub effect DuplexStream
	read_chunk() -> Option<ByteChunk>
	write_chunks(chunks: List<ByteChunk>) -> ()
end
```

The exported `transport::net` module declares
`net_stream(stream: NetStream)` as a public lexical handler for
`transport::DuplexStream`. Handling a body with `transport::net::net_stream`
removes `std::transport::DuplexStream` from the inferred effect set and adds
the existing coarse `net` effect from the handler clauses. A public function
that performs a duplex-stream operation without a handler must declare the
duplex-stream effect. A public function that wraps that body with the
`net_stream` handler must declare `net` and does not retain the handled
duplex-stream effect. The static boundary is checked by
`examples/specification/check/http2-connection-transport-handler-effects/`.
`http2::connection::drive_server` and `http2::connection::drive_client` expose
only `std::transport::DuplexStream`; handling either driver with `net_stream`
therefore replaces that nominal effect with `net` and does not expose
HTTP/2-internal effects.
`http2::connection::drive_server_application<effect E>` exposes
`std::transport::DuplexStream` plus the effect row of its application
callback. The callback type is
`fn(Http2ApplicationEvent) -> Result<List<Http2ApplicationAction>, String>
effects [...E]`, and the driver requires
`[std::transport::DuplexStream, ...E]`. Handling that driver with
`transport::net::net_stream` removes only the duplex-stream effect and leaves
callback effects such as `db` on the handled expression. The static boundary
is checked by
`examples/specification/check/http2-service-transport-effect-replacement/`.

## Stdio Calls

The implemented compiler-known stdio calls are registered in the standard
symbol table. The current stdio entries are:

```veln
stdio::print(text: String) -> () effects [stdio]
stdio::println(text: String) -> () effects [stdio]
stdio::eprint(text: String) -> () effects [stdio]
stdio::eprintln(text: String) -> () effects [stdio]
```

Direct calls to these functions infer the `stdio` effect. Function signatures
also carry effects inferred from their bodies, so a public function or test that
calls a private helper whose body reaches `stdio` must declare `stdio` even when
the helper omitted its own `effects` clause. Function-body effect inference
follows direct bare function calls and `use` alias qualified function calls
until a fixed point. Public function aliases carry the referenced function's
signature and effects. Calls through a local binding with a function type infer
the effects written in that function type.

## File System Calls

The checker recognizes these file-system call targets through the standard
symbol table:

```veln
fs::read_to_string(path: Path) -> Result<String, FsError> effects [fs]
fs::write_string(path: Path, text: String) -> Result<(), FsError> effects [fs]
fs::exists(path: Path) -> Result<Bool, FsError> effects [fs]
fs::read_dir(path: Path) -> Result<Vec<Path>, FsError> effects [fs]
```

Direct calls to these functions infer the `fs` effect. A public function or
test that calls one of them directly or through a private helper must declare
`fs` in its `effects [...]` list.

`Path` is a source-visible named type at this boundary. Runtime path values are
backend-owned values that can be passed between implemented `fs` and `process`
calls, but assignment compatibility does not allow `String` and `Path` to cross
this boundary. The language does not expose a public path layout, encoding, or
normalization guarantee.

File-system calls return `Result` values instead of throwing host I/O
exceptions into Veln execution. `Ok` carries the successful value. `Err`
carries an implementation-provided `FsError` value represented by the current
runtime error text.

## Network And Time Boundary Calls

The checker recognizes these minimal transport-boundary call targets through
the standard symbol table:

```veln
net::receive_chunk() -> ByteChunk effects [net]
net::send_chunk(bytes: ByteChunk) -> () effects [net]
net::listen(address: String) -> NetListener effects [net]
net::connect(address: String) -> NetStream effects [net]
net::accept(listener: NetListener) -> NetStream effects [net]
net::accept_or_end(listener: NetListener) -> Option<NetStream> effects [net]
net::accept_until(listener: NetListener, deadline: Deadline) -> Option<NetStream> effects [net, time]
net::accept_until_cancellable(listener: NetListener, deadline: Deadline, token: CancelToken) -> AcceptOutcome effects [net, time]
net::listener_local_addr(listener: NetListener) -> String effects [net]
net::read_chunk(stream: NetStream) -> ByteChunk effects [net]
net::stream_local_addr(stream: NetStream) -> String effects [net]
net::stream_peer_addr(stream: NetStream) -> String effects [net]
net::stream_can_read(stream: NetStream) -> Bool effects [net]
net::stream_can_write(stream: NetStream) -> Bool effects [net]
net::stream_is_closed(stream: NetStream) -> Bool effects [net]
net::read_chunk_until(stream: NetStream, deadline: Deadline) -> Option<ByteChunk> effects [net, time]
net::read_chunk_until_cancellable(stream: NetStream, deadline: Deadline, token: CancelToken) -> StreamReadOutcome effects [net, time]
net::read_chunk_or_end(stream: NetStream) -> Option<ByteChunk> effects [net]
net::write_chunk(stream: NetStream, bytes: ByteChunk) -> () effects [net]
net::write_chunk_until(stream: NetStream, bytes: ByteChunk, deadline: Deadline) -> StreamWriteOutcome effects [net, time]
net::write_chunk_until_cancellable(stream: NetStream, bytes: ByteChunk, deadline: Deadline, token: CancelToken) -> StreamWriteOutcome effects [net, time]
net::write_chunks(stream: NetStream, chunks: List<ByteChunk>) -> () effects [net]
net::write_chunks_until(stream: NetStream, chunks: List<ByteChunk>, deadline: Deadline) -> StreamWriteOutcome effects [net, time]
net::write_chunks_until_cancellable(stream: NetStream, chunks: List<ByteChunk>, deadline: Deadline, token: CancelToken) -> StreamWriteOutcome effects [net, time]
net::shutdown_write(stream: NetStream) -> () effects [net]
net::shutdown_read(stream: NetStream) -> () effects [net]
net::close_stream(stream: NetStream) -> () effects [net]
net::close_listener(listener: NetListener) -> () effects [net]
time::monotonic_ms() -> Int effects [time]
time::timeout_ms(milliseconds: Int) -> () effects [time]
time::deadline_after_ms(milliseconds: Int) -> Deadline effects [time]
time::deadline_at_ms(target_ms: Int) -> Deadline effects [time]
time::wait_until(deadline: Deadline) -> () effects [time]
time::cancel_token() -> CancelToken effects [time]
time::cancel_owner() -> CancelOwner effects [time]
time::cancel_token_from(owner: CancelOwner) -> CancelToken effects [time]
time::cancel_owned(owner: CancelOwner) -> () effects [time]
time::cancel(token: CancelToken) -> () effects [time]
time::is_cancelled(token: CancelToken) -> Bool effects [time]
time::is_cancelled_owner(owner: CancelOwner) -> Bool effects [time]
time::wait_until_cancellable(deadline: Deadline, token: CancelToken) -> () effects [time]
time::wait_until_cancellable_outcome(deadline: Deadline, token: CancelToken) -> CancellableWaitOutcome effects [time]
```

Direct calls to `net::receive_chunk` and `net::send_chunk` infer the `net`
effect. Direct calls to `net::listen`, `net::connect`, `net::accept`,
`net::accept_or_end`, `net::listener_local_addr`, `net::read_chunk`,
`net::stream_local_addr`, `net::stream_peer_addr`,
`net::stream_can_read`, `net::stream_can_write`,
`net::stream_is_closed`, `net::read_chunk_or_end`, and
`net::write_chunk`, `net::write_chunks`, `net::shutdown_write`,
`net::shutdown_read`, `net::close_stream`, and `net::close_listener` also
infer the same coarse `net` effect. Direct calls
to `net::accept_until`,
`net::read_chunk_until`, and
`net::read_chunk_until_cancellable`,
`net::write_chunk_until`,
`net::write_chunk_until_cancellable`,
`net::write_chunks_until`, and
`net::write_chunks_until_cancellable` infer both `net` and `time` because
the adapter-owned accept, read, or write attempt observes a `Deadline` or
`CancelToken`.
Direct calls to
`stream_adapter_drain_actions_until_cancellable` infer `net`, `time`, and
`concurrency` because the helper owns stream I/O, deadline/cancellation
observation, and channel routing.
Direct calls to `time::timeout_ms`,
`time::monotonic_ms`, `time::deadline_after_ms`,
`time::deadline_at_ms`, `time::wait_until`,
`time::cancel_token`,
`time::cancel_owner`, `time::cancel_token_from`,
`time::cancel_owned`,
`time::cancel`, `time::is_cancelled`, `time::is_cancelled_owner`, and
`time::wait_until_cancellable`,
`time::wait_until_cancellable_outcome` infer the `time` effect. A public
function or test that calls one of them directly or through a private helper
must declare the matching effect in its `effects [...]` list.

This boundary is intentionally narrow. `net::receive_chunk`
returns a host-fed immutable `ByteChunk`; `net::send_chunk` exposes an outgoing
chunk to the host runtime; `net::listen` returns a source-visible
`NetListener`; `net::connect` and `net::accept` return distinct
source-visible `NetStream` handles.
The default runtime path remains fixture-backed: `net::connect(address)`
records a client connection attempt and returns an owned stream whose peer
endpoint text is the requested address; `net::accept_or_end`
returns `Some(stream)` for a fixture-accepted stream and `None` when the
fixture listener reaches a clean end; `net::accept_until`
returns `Some(stream)` when a fixture accepts before the deadline and `None`
when the deadline has already expired or the fixture reports deadline expiry
before accepting; `net::read_chunk`
reads one immutable `ByteChunk` from that stream;
`net::read_chunk_until` returns `Some(bytes)` when the fixture stream yields
a chunk before the deadline and `None` when the deadline has already expired,
the fixture reports deadline expiry before a chunk is read, or the fixture
stream reaches a clean end before a chunk is read;
`net::read_chunk_until_cancellable` returns `ReadChunk(bytes)` when the
fixture stream yields a chunk before the deadline and before cancellation,
`ReadEnd` for clean stream end, `ReadDeadlineExpired` for supplied or
fixture-reported read deadline expiry, and `ReadCancelled` when the supplied
`CancelToken` has been cancelled;
`net::read_chunk_or_end` returns `Some(bytes)` for a successful stream read
and `None` when the fixture stream reaches a clean end; and `net::write_chunk`
writes one immutable `ByteChunk` to that stream.
`net::write_chunk_until` returns `WriteCompleted` after writing one immutable
`ByteChunk` before the deadline and `WriteDeadlineExpired` for supplied or
fixture-reported write deadline expiry.
`net::write_chunk_until_cancellable` returns `WriteCompleted` after writing
one immutable `ByteChunk` before the deadline and before cancellation,
`WriteDeadlineExpired` for supplied or fixture-reported write deadline expiry,
and `WriteCancelled` when the supplied `CancelToken` has been cancelled.
`net::write_chunks` writes a
source-owned `List<ByteChunk>` to the same stream in list order.
`net::write_chunks_until` writes that list in source order, returns
`WriteCompleted` after every chunk is written before the deadline, and returns
`WriteDeadlineExpired` when deadline expiry wins before the list is fully
written.
`net::write_chunks_until_cancellable` writes that list in source order,
returns `WriteCompleted` after every chunk is written before the deadline and
before cancellation, returns `WriteDeadlineExpired` when deadline expiry wins
before the list is fully written, and returns `WriteCancelled` when the
supplied `CancelToken` wins before the list is fully written.
`net::shutdown_write` records fixture-backed adapter-owned write-side
shutdown, returns `()`, and leaves clean read end on the existing
`net::read_chunk_or_end` path. Later writes on the same stream fail as runtime
transport failures.
`net::shutdown_read` records fixture-backed adapter-owned read-side shutdown,
returns `()`, and makes later optional stream reads observe clean end while
leaving the write side owned by the same `NetStream`.
`net::close_stream` records fixture-backed adapter-owned stream cleanup and
returns `()`. `net::close_listener` records fixture-backed adapter-owned
listener cleanup and returns `()`; after that close, `net::accept`,
`net::accept_or_end`, `net::accept_until`, and
`net::accept_until_cancellable` fail as runtime transport failures instead of
reporting clean end, deadline expiry, or cancellation.
Connected and accepted streams expose endpoint text through
`net::stream_local_addr` and `net::stream_peer_addr`, expose read-capable,
write-capable, and closed status through `net::stream_can_read`,
`net::stream_can_write`, and `net::stream_is_closed`, and use the same read,
write, write-side shutdown, read-side shutdown, and close helpers. State
inspection returns `Bool` values without consuming stream ownership. Forced
connection failure remains a runtime transport failure.
Fixture-backed listeners expose their local endpoint text through
`net::listener_local_addr` before accept work without exposing host socket
handles, closing the listener, or changing later accepted streams.
When `VELN_NET_RUNTIME` is `production-loopback`, the same public calls own a
host loopback listener and deterministic loopback stream sequence:
`net::listen` binds the requested host and port,
`net::listener_local_addr` reports the bound listener endpoint text,
`net::connect` returns a
deterministic client-side loopback stream, `net::accept` and
`net::accept_or_end` accept a loopback client as a `NetStream`,
`net::accept_until` accepts before the supplied deadline or reports clean
listener end as `None`, `net::read_chunk` and `net::read_chunk_or_end` read
bytes from that stream, `net::read_chunk_until` reads bytes before the
supplied deadline or reports clean stream end as `None`, `net::write_chunk`
writes bytes back to the stream, `net::write_chunks` writes each chunk in
source list order, `net::shutdown_write` shuts down the stream write side
without replacing the read clean-end path, `net::shutdown_read` shuts down the
stream read side so later optional reads report clean end while the write side
can still write, `net::stream_can_read`, `net::stream_can_write`, and
`net::stream_is_closed` observe the stream state before and after those
shutdowns and after full close, `net::close_stream` closes the owned stream,
state inspection can still observe the closed handle, later read, write, or
shutdown transport operations on that stale handle fail as runtime transport
failures, and a following optional or deadline-aware accept can observe clean
listener end.
When a source program opens a production-loopback listener and then calls
`net::connect` with the same source-visible address value while that listener
is still open, the client stream is paired with that listener. A following
`net::accept` or `net::accept_or_end` returns the server-side `NetStream`;
both handles are source-owned, both use the same read, write, endpoint,
write-side shutdown, and close helpers, and both must be closed explicitly by
source code that owns them. The runtime records the listener, client connect,
accept, byte reads and writes, stream closes, clean listener end, and listener
close on the same production event path. Connection, accept, read, write, and
close failures remain runtime transport failures.
When `VELN_NET_RUNTIME` is `external`, `net::listen` owns a host listener
without starting a synthetic client, and `net::connect` opens a host connection
without consulting the runtime's listener registry. Accepted and connected
host sockets remain encapsulated by `NetListener` and `NetStream`; source code
uses the existing endpoint inspection, read, write, shutdown, deadline,
cancellation, state inspection, and close calls under the same coarse `net`,
`time`, and `concurrency` effects. Bind and connection failures retain the
structured transport payload and do not create fixture or in-memory handles.
The backend external-peer integration tests check independently owned host
clients and listeners, while the focused
`transport-socket-external-*-failure-*` run cases check human and JSON failure
details.
`net::close_listener` closes the owned production listener or in-memory
loopback listener state without closing already accepted `NetStream` handles;
any later accept call on that listener fails through the same runtime
transport boundary. Production-loopback connected streams use the same
endpoint, read, write, shutdown, and close lifecycle as accepted production
streams.
Adapter-owned production loopback examples can handle multiple accepted
streams independently through ordinary `StreamInput` and response-action
values, route them through the existing `concurrency` boundary, project only
ordered `SendBytes` actions to `net::write_chunk`, close each stream, and
drain the listener until clean end. A production multi-chunk routing case
keeps configured read chunk boundaries within one accepted stream, exposes
each read as an ordinary `StreamInput.Chunk` routed through the same channel
boundary, calls a pure handler for each chunk and clean end, and projects the
ordered `SendBytes` response actions through `net::write_chunks` under the
same coarse `net` and `concurrency` effects. The multi-event adapter
task-helper variant routes those stream events through the same channel
boundary and an adapter-owned task helper. That helper carries adapter-owned
route and trace metadata through `task::spawn_with<Result, Context>`,
preserves event sequence, and calls the pure handler without exposing
`NetStream` access. A companion per-stream handler-failure case treats a
handler-returned `Err` from that task boundary as an ordinary
adapter-owned action value, closes the accepted stream, observes clean
listener end, and does not call `net::write_chunks` for that failed stream.
Its matching static effect case rejects adapter entry points that omit either
label while leaving the public handler boundary effect-free. A forced
production read failure on that same
multi-chunk routing path remains a runtime transport failure after production
accept and before any chunk routing, response writes, stream close, or clean
listener end is recorded. The deadline-aware production adapter
uses the same handler/action boundary through `net::accept_until` and
`net::read_chunk_until`, adds only the existing coarse `time` effect label,
writes ordered response bytes, closes the stream, and then observes clean
listener end through a following deadline-aware accept. Forced production read
failure on the listener-drain path, and forced production accept or read
failure through the deadline-aware paths, remain runtime transport failures.
This production path uses the same coarse `net` and `time` effects as the
fixture-backed deadline-aware path.
`time::timeout_ms` waits at the runtime boundary; `time::deadline_after_ms`
creates a relative `Deadline`; `time::deadline_at_ms` creates a `Deadline`
from an absolute monotonic millisecond value in the same host-owned clock
domain returned by `time::monotonic_ms`; `time::monotonic_ms` returns a
host-owned monotonic millisecond counter for elapsed-time measurement without
exposing wall-clock dates; `time::wait_until` waits until that deadline
expires;
`time::cancel_token` returns a source-visible cancellation handle;
`time::cancel_owner` returns a source-visible cancellation owner;
`time::cancel_token_from` exposes an observer `CancelToken` from that owner
for existing cancellable wait, channel, and socket calls;
`time::cancel_owned` requests cancellation through the owner while preserving
the observer token API;
`time::cancel` requests cancellation through a direct token created by
`time::cancel_token`; attempting to cancel an owner-derived observer token
through `time::cancel` is a runtime failure; `time::is_cancelled`
observes token state as `Bool` without waiting, cancelling, allocating a new
handle, or reporting a runtime transport failure; `time::is_cancelled_owner`
observes owner state through the same underlying cancellation state without
waiting, cancelling, allocating a new observer token, or changing direct-token
compatibility; and
`time::wait_until_cancellable` waits until a deadline expires unless the
handle is cancelled first.
`time::wait_until_cancellable_outcome` uses the same deadline and token values
and returns `WaitCompleted`, `WaitDeadlineExpired`, or `WaitCancelled` as an
ordinary `CancellableWaitOutcome` value for adapter-owned branching. Malformed
host-fed receive or read bytes, failed outgoing send, write, stream close, or
listener close event recording, forced listen, accept, read, write, close,
timeout, deadline expiry through runtime-failure waits, or cancellable-wait
cancellation failures through the runtime-failure wait are transport runtime
failures, not schema, codec, or peer protocol diagnostics.
`net::accept_until_cancellable` returns
`AcceptStream(stream)` for an accepted stream, `AcceptEnd` for clean listener
end, `AcceptDeadlineExpired` for accept deadline expiry, and `AcceptCancelled`
for token cancellation. Forced accept failure through `net::accept_until`
or `net::accept_until_cancellable`
and forced read failure through `net::read_chunk_until` or
`net::read_chunk_until_cancellable` remain runtime failures; only deadline
expiry reported by those optional paths becomes `None` or
`ReadDeadlineExpired`, and token cancellation through the cancellable read
path becomes `ReadCancelled`.
These calls do not define stream routing, richer timer handles beyond
`Deadline` and `CancelToken`, TLS, ALPN, or an HTTP application framework.
The checked stream adapter cancellable routing cases use
`time::wait_until_cancellable_outcome` before returning ordinary response
action values from channel-routed `StreamInput` handling. The receiver-list
cancellable channel-first case uses
`channel::select_many_timeout_cancellable` over a
`List<Receiver<StreamInput>>`, translating `Ok(Some(selected))`,
`Ok(None)`, and `Err(SelectError)` into routed, timed-out, and cancelled
source outcome values before producing adapter actions. The adapter declares
both `time` and `concurrency`; a socket-owning wrapper around the same helper
declares `net`, `time`, and `concurrency`; and the pure handler it calls
remains free of transport effects.
The cancellation-owner lifecycle case uses `time::cancel_owner`,
`time::cancel_token_from`, and `time::cancel_owned` so adapter cleanup keeps
the cancellation owner while routing and socket code receive only the
observer `CancelToken`. After cleanup requests cancellation through the owner,
`time::wait_until_cancellable_outcome` returns `WaitCancelled` and
`net::read_chunk_until_cancellable` returns `ReadCancelled` as ordinary
adapter-observable outcome values under the same `net`, `time`, and
`concurrency` boundary. The observer-only runtime case keeps direct
`time::cancel` from taking authority through an owner-derived token.

The implemented socket stream adapter routing examples compose multiple
socket reads and ordered `net::write_chunk` calls with standard channel and
task calls. One case uses `net::read_chunk` for byte-only reads; the clean-end
case uses `net::read_chunk_or_end` so adapter-owned source can translate
`None` into the standard `StreamInput.End` value for a pure handler boundary.
The production multi-chunk routing case uses the same optional read surface to
turn more than one host-owned read chunk from one accepted stream into
ordinary `StreamInput.Chunk` values before clean end, then writes only ordered
`SendBytes` response actions through `net::write_chunks`. A companion
multi-event adapter task-helper case sends each routed event through an
adapter-owned task helper using `task::spawn_with<Result, Context>` with
adapter-owned route and trace metadata, preserving trace identity and sequence
across multiple events from the same accepted stream while the pure handler
stays outside the task and channel effect boundary. A per-stream
handler-failure companion case returns `Err` from that task-owned handler,
converts it to an ordinary source-visible adapter action, performs
adapter-owned stream cleanup, and skips response-byte projection for the
failed stream. Its matching effect case rejects adapter paths that omit either
`net` or `concurrency` while leaving the handler boundary effect-free. Forced
read failure on the same
optional-read routing path remains a runtime transport failure before any
ordinary `StreamInput.Chunk` is routed or response bytes are written.
The standard helper `stream_adapter_drain_actions` exposes this adapter-level
shape as a reusable boundary over one accepted `NetStream`: it drains optional
stream reads through a channel-routed `StreamInput` boundary, calls a pure
`fn(StreamInput) -> List<StreamAdapterAction>` handler, preserves ordered
handler actions, and writes only `SendBytes` chunks through
`net::write_chunks` under `net` and `concurrency`.
The cancellable helper `stream_adapter_drain_actions_until_cancellable`
exposes the same adapter-level drain, route, and pure handler boundary, then
projects only ordered `SendBytes` chunks through
`net::write_chunks_until_cancellable`. It returns `WriteCompleted`,
`WriteDeadlineExpired`, or `WriteCancelled` as ordinary
`StreamWriteOutcome` values, preserves host write failures as runtime
transport failures, and requires `net`, `time`, and `concurrency`.
The owned-lifecycle case accepts a listener with `net::accept_or_end`, owns the
accepted stream through repeated optional reads, routes ordinary stream values
through a channel, calls the plain handler without exposing socket handles, and
projects `SendBytes` actions back into ordered `net::write_chunk` calls. The
deadline-aware lifecycle case accepts with `net::accept_until`, owns the
accepted stream through repeated `net::read_chunk_until` attempts, translates
deadline expiry into the ordinary stream boundary value before calling the
plain handler, and projects only `SendBytes` actions to ordered writes. The
cancellable deadline-aware lifecycle case accepts with
`net::accept_until_cancellable`, owns the accepted stream in adapter code,
reads through `net::read_chunk_until_cancellable`, translates accept and read
clean-end, deadline, and cancellation outcomes into adapter decisions or
ordinary response actions, and projects only `SendBytes` actions to ordered
writes. The
matching owned-lifecycle effect check rejects adapter paths that omit either
`net` or `concurrency`; the matching cancellable deadline-aware lifecycle
effect check rejects adapter paths that omit `net`, `time`, or
`concurrency`. The non-deadline adapter functions declare both `net` and
`concurrency`; the deadline-aware lifecycle adapter declares `net`, `time`,
and `concurrency`, and the cancellable deadline-aware lifecycle adapter
declares the same three effects.
The plain handlers they call remain free of socket handles and `net` calls.
This composition does not add any effect label beyond the existing coarse
labels or any compiler-known routing symbol beyond the socket, channel, task,
and deadline calls listed here.

The channel-first stream routing examples route ordinary `StreamInput` values
through typed channel routes, select a ready route with existing channel
selection, and then invoke a plain handler with explicit per-stream state. The
general receiver-list helper example accepts a non-empty
`List<Receiver<StreamInput>>`, exercises more than four routes, and returns
the selected route index plus value. The receiver-list priority routes use
`channel::select_many_priority` on a non-empty
`List<Receiver<StreamInput>>` and preserve supplied list order as priority
order. The timeout route uses `channel::select_many_timeout` with the same
list priority and returns `None` when no receiver is ready before the timeout.
The routing adapter declares `concurrency`; the cancellable channel-first
adapter calls `channel::select_many_timeout_cancellable` and declares both
`time` and `concurrency`; and socket wrappers around cancellable routing
declare `net`, `time`, and `concurrency`. The handler itself remains free of
transport effects.

## Process Calls

The checker recognizes these current-process call targets through the standard
symbol table:

```veln
process::args() -> Vec<String> effects [process]
process::env(name: String) -> Option<String> effects [process]
process::cwd() -> Result<Path, ProcessError> effects [process]
process::exit(status: Int) -> () effects [process]
```

Direct calls to these functions infer the `process` effect. A public function
or test that calls one of them directly or through a private helper must
declare `process` in its `effects [...]` list.

`process::env` returns `None` for unavailable environment keys.
`process::cwd` returns `Ok(path)` for the current working directory or
`Err(ProcessError)` when the runtime cannot produce one. `process::exit`
terminates the selected program through the host runtime after clamping the
status into the implemented backend status range.

## Concurrency Calls

The checker recognizes these channel-operation call targets through the
standard symbol table for effect metadata, and through the existing
concurrency signature rules for static type checking:

```veln
channel::bounded(capacity: Int) -> {tx: Sender<T>, rx: Receiver<T>} effects [concurrency]
channel::bounded<T>(capacity: Int) -> {tx: Sender<T>, rx: Receiver<T>} effects [concurrency]
channel::clone(tx: Sender<T>) -> Sender<T> effects [concurrency]
channel::send(tx: Sender<T>, value: T) -> Result<(), SendError> effects [concurrency]
channel::recv(rx: Receiver<T>) -> Option<T> effects [concurrency]
channel::select(left: Receiver<T>, right: Receiver<T>) -> Option<{index: Int, value: T}> effects [concurrency]
channel::select_priority(left: Receiver<T>, right: Receiver<T>) -> Option<{index: Int, value: T}> effects [concurrency]
channel::select_many_priority(receivers: List<Receiver<T>>) -> Option<{index: Int, value: T}> effects [concurrency]
channel::select_many_timeout(receivers: List<Receiver<T>>, timeout_ms: Int) -> Option<{index: Int, value: T}> effects [concurrency]
channel::select_many_timeout_result(receivers: List<Receiver<T>>, timeout_ms: Int) -> Result<Option<{index: Int, value: T}>, SelectError> effects [concurrency]
channel::select_many_timeout_cancellable(receivers: List<Receiver<T>>, timeout_ms: Int, token: CancelToken) -> Result<Option<{index: Int, value: T}>, SelectError> effects [time, concurrency]
channel::select_timeout(left: Receiver<T>, right: Receiver<T>, timeout_ms: Int) -> Option<{index: Int, value: T}> effects [concurrency]
channel::select_timeout_cancellable(left: Receiver<T>, right: Receiver<T>, timeout_ms: Int, token: CancelToken) -> Result<Option<{index: Int, value: T}>, SelectError> effects [time, concurrency]
channel::select_result(left: Receiver<T>, right: Receiver<T>) -> Result<Option<{index: Int, value: T}>, SelectError> effects [concurrency]
channel::select_priority_result(left: Receiver<T>, right: Receiver<T>) -> Result<Option<{index: Int, value: T}>, SelectError> effects [concurrency]
channel::select_timeout_result(left: Receiver<T>, right: Receiver<T>, timeout_ms: Int) -> Result<Option<{index: Int, value: T}>, SelectError> effects [concurrency]
channel::close(tx: Sender<T>) -> () effects [concurrency]
```

Direct calls to these functions infer their listed effects. A public function
or test that calls one of them must declare those effects in its
`effects [...]` list. The cancellable receiver-list timeout helper is the
only channel helper in this set that also infers `time`, because observing its
`CancelToken` is a time-boundary operation.

`channel::bounded(capacity)` creates a bounded channel pair. Its item type is
inferred from the expected record type, such as
`{tx: Sender<String>, rx: Receiver<String>}`. `channel::bounded<T>(capacity)`
uses the explicit item type when no expected record type is present.
`channel::clone` returns another sender endpoint for the same channel and
preserves the sender item type. `channel::send` waits while a positive-capacity
channel is full, returns `Ok(())` when the value is queued or transferred
through a zero-capacity rendezvous, and returns `Err(SendError)` when the
sender cannot accept the value. `channel::recv` waits for a queued value, a
rendezvous value, or sender close, returns `Some(value)` for a received value,
and returns `None` after the channel is closed and drained. A zero-capacity
channel has no queue storage; a send waits until a receiver is ready and then
transfers the value directly.
`channel::select(left, right)` waits for either receiver to produce a value or
close. If a value is available, it returns `Some({index, value})`; `index` is
`0` for the left receiver and `1` for the right receiver. When both receivers
are closed and drained, it returns `None`. When both receivers are ready in the
same poll, repeated selections rotate the first polled receiver so ties
alternate between index `0` and index `1`.
`channel::select_priority(left, right)` has the same receiver and return typing
as `channel::select`, but when both receivers are ready in the same poll the
left receiver wins.
`channel::select_many_priority(receivers)` accepts a non-empty
`List<Receiver<T>>` and returns `Some({index, value})` with the zero-based
receiver index from that list. When multiple receivers are ready in the same
poll, the earliest receiver in the supplied list wins. It returns `None` after
all supplied receivers are closed and drained.
`channel::select_many_timeout(receivers, timeout_ms)` has the same receiver
list and return typing as `channel::select_many_priority`, plus an `Int`
millisecond timeout. It preserves supplied list order as priority order and
returns `None` when no supplied receiver has a ready value before the timeout
elapses. Negative timeouts wait without a timeout.
`channel::select_many_timeout_result(receivers, timeout_ms)` has the same
receiver list, priority order, and timeout behavior as
`channel::select_many_timeout`, but returns `Ok(Some(selected))` for a
selected value, `Ok(None)` for closed or timed-out selection, and
`Err(SelectError)` when cooperative cancellation interrupts the waiting
selection.
`channel::select_many_timeout_cancellable(receivers, timeout_ms, token)` has
the same receiver list, priority order, timeout behavior, and selected value
shape as `channel::select_many_timeout_result`. It returns
`Ok(Some(selected))` for the first ready receiver in supplied list order,
`Ok(None)` when the timeout elapses or all supplied receivers close before a
value is selected, and `Err(SelectError)` when the supplied `CancelToken`
is already cancelled or becomes cancelled before a ready receiver wins. The
checked cancellable channel-first adapter maps that result into an ordinary
source route outcome so cancellation is a visible adapter completion case
instead of another fixed route-count fixture.
`channel::select_timeout(left, right, timeout_ms)` has the same receiver and
return typing as `channel::select`, plus an `Int` millisecond timeout. It
returns `None` when the timeout elapses before a value is selected. Negative
timeouts wait without a timeout.
`channel::select_timeout_cancellable(left, right, timeout_ms, token)` has the
same receiver order, rotating tie behavior, timeout behavior, and selected
result shape as `channel::select_timeout_result`. It returns
`Ok(Some(selected))` for a selected value, `Ok(None)` when the timeout elapses
or both receivers close before a value is selected, and `Err(SelectError)`
when the supplied `CancelToken` is already cancelled or becomes cancelled
before a ready receiver wins.
`channel::select_result`, `channel::select_priority_result`, and
`channel::select_timeout_result` use the same selection rules as their
non-result counterparts with the same result boundary.
`channel::close` closes the sender endpoint, wakes waiting receivers, and
returns `()`.

The checker also recognizes these task-operation call targets:

```veln
task::spawn<T, effect E>(job: fn() -> T effects [...E]) -> Task<T> effects [concurrency, ...E]
task::spawn_with<T, C, effect E>(job: fn(C) -> T effects [...E], context: C) -> Task<T> effects [concurrency, ...E]
task::join(task: Task<T>) -> Result<T, JoinError> effects [concurrency]
task::cancel(task: Task<T>) -> () effects [concurrency]
```

`task::spawn` starts a zero-argument callable in a concurrent task and returns
its task handle. `task::spawn_with` starts a one-argument callable with one
ordinary source context value. A pure job substitutes the empty effect set for
`E`, so the task creation expression infers only `concurrency`. An effectful
job substitutes its concrete effect set for `E`, so the task creation
expression infers `concurrency` plus each job effect once. A job that handles
`transport::DuplexStream` with `transport::net::net_stream(stream)` substitutes
the handled expression's remaining effects plus `net`; the task creation
expression does not retain `transport::DuplexStream` after handler
replacement. The checked `http2-service-task-effect-row` case fixes the pure
zero-argument, effectful zero-argument, and context-job boundaries.

The optional first explicit type argument fixes the task item type, and the
optional second explicit type argument fixes the context parameter type for
`task::spawn_with<T, C>(job, context)`. The context value may be an anonymous
record or any existing named type accepted by the handler. Numbered
multi-argument task spawn calls are not standard task operations; callers
carry multiple values through one context argument. Arguments are frozen
before crossing into the task, and the result value is frozen before it
crosses back through the task handle.
`task::join` waits for completion and returns `Ok(value)` when the task returns
normally, or `Err(JoinError)` when the task is interrupted, cancelled, or fails
at runtime. `task::cancel` requests cancellation by interrupting the task and
returns `()`. Cancellation is
cooperative at the JVM runtime boundary.

Executable-command reachability also follows bare and `use`-alias qualified
function declaration values in reachable expressions, public function aliases,
pure helper calls used in reachable contract predicates, and function
declaration values passed as contract call arguments. Calls through
function-typed local bindings and parameters conservatively include visible
same-arity function declarations when the surface graph does not identify one
concrete target, so blockers inside possible helpers are reported before the
selected entry runs.

A public function whose declared effects omit an inferred effect reports
`effect.missing_public` with related provenance pointing at bounded call sites.
Effect diagnostics include bounded structured provenance paths. Each path
records the boundary entry, the effect-causing call entry, whether the path set
was truncated, how many frames were hidden, and how many equivalent paths were
omitted. For the current direct-call, signature-based, and body-inferred helper
inference, hidden frame counts are zero.
