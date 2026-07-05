# Names And Effects

This file specifies implemented name resolution and effect checking.

## Name Resolution

Implemented checker namespaces are:

- module imports
- value declarations, including functions, parameters, and `let` bindings
- record fields inside one record literal

Bare names resolve to local bindings. Function calls resolve to:

- compiler-known stdio calls
- local bindings with function type
- declarations in the current source module by bare name
- unambiguous public function exports from written imports by bare name
- discovered function signatures through a `use` alias in `alias::function`
  form
- source path derived local imports through their full written module path in
  `module::path::function` form
- public function aliases through the declaring module path
- implicit standard prelude helper imports by bare name or `prelude::function`
  form

Unresolved values and call targets produce `name.unresolved` diagnostics. A
qualified call does not fall back to a bare function with the same final
segment when no matching import alias exists.
When more than one import provides the same bare function name, including a
conflict between a written import and the implicit prelude import, the checker
reports `name.ambiguous` at the bare name and lists qualified spellings in
related notes.
Duplicate declarations in the same implemented namespace produce
`name.duplicate` diagnostics at the later declaration, with the first
declaration reported as related context.

Local value bindings and declarations in the current source module shadow
imported names for both bare values and calls. The standard prelude remains
available through `prelude::` when a local declaration shadows its bare name.
The `StreamInput` standard ADT constructors are available as `Chunk(bytes)`,
`End`, `StreamInput::Chunk(bytes)`, `StreamInput::End`,
`prelude::Chunk(bytes)`, `prelude::End`,
`prelude::StreamInput::Chunk(bytes)`, and `prelude::StreamInput::End`.
The `AcceptOutcome` standard ADT constructors are available through the same
bare, type-qualified, prelude-qualified, and prelude-type-qualified forms.

A wildcard let target, `_`, evaluates its expression without declaring a local
name. It can be annotated for type checking, but it is never a resolvable
binding.

Current duplicate checks reject:

- duplicate import paths within the same source module
- duplicate top-level function, test, or public function alias names
- duplicate top-level source type or public type alias names
- duplicate parameter names in one function
- a result binding that duplicates a parameter name
- duplicate `let` names in the same function value scope, including names that
  duplicate parameters
- duplicate field names in one record literal
- duplicate pattern binding names in one match arm, including names that
  duplicate bindings already visible at the arm
- duplicate field names in one record pattern

Record type annotations also require unique field names. Duplicate record type
fields are reported through invalid type annotation diagnostics because they are
part of annotation parsing rather than value-name resolution.

For selected package-relative sources, the command analysis path derives local
module identity from the source path before semantic checks run. Written
imports are scoped to the source module that declares them. Bare public imports
and qualified module paths from another same-package module are visible only in
that declaring source module. User source cannot derive module identity
`prelude` or write an import path whose alias is `prelude`; both names are
reserved for the implicit standard prelude import and report `name.reserved`.

External `use path from "package"` declarations resolve `path` inside an
already available path dependency whose dependency table key is `package`.
The dependency manifest's `[package].name` must match that package identity,
and external modules are importable only when their derived source module path
is listed by the dependency package's `[lib].exports`. The import exposes only
public declarations and public aliases from that exported module; private names
remain private even when the dependency source is loaded for analysis.

When `veln.toml` contains manifest export data, `[modules]` is rejected and
`[lib].exports` is checked as a list of public package-relative source files.
Export entries must be selected source files, must use `.veln` file-path
spelling instead of module paths, must stay inside the package, and must derive
unique source module paths.

Named holes remain repair labels, not value declarations. Reusing a hole label
does not affect name resolution.

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
function or test declaration. The checker currently infers `stdio`, `fs`,
`net`, `time`, `process`, and `concurrency` from compiler-known calls. The
other labels are reserved coarse-grained public boundary labels for source
compatibility.

## Compiler-Known Descriptor Table

Semantic analysis owns a standard symbol table for compiler-known library
symbols. The table records the source-visible module, name, symbol kind, effect
labels, lowering identity, and stability class for the descriptor-backed
subset.

The current descriptor-backed subset covers stdio effect metadata,
concurrency effect metadata, minimal `fs`, `net`, `time`, and `process`
intrinsics, pure prelude helper admission, and source provenance for
source-backed pure helpers. Type adapters and most runtime lowering still use
their existing specialized implementations.

For prelude helpers, the descriptor table is also the source of truth for
whether a helper is descriptor-only or source-backed. A source-backed helper
records embedded source metadata on its descriptor; descriptor-only helpers do
not.

The implemented standard library source subset also includes a small
`compiler_support` source-loading helper used as the compiler-subsystem trial
for self-hosting work. It is checked and run by the test suite against the same
descriptor-backed `fs` boundary available to user source.

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
`net::read_chunk_or_end`, and
`net::write_chunk`, `net::write_chunks`, `net::shutdown_write`,
`net::close_stream`, and `net::close_listener` also infer the same coarse
`net` effect. Direct calls
to `net::accept_until`,
`net::read_chunk_until`, and
`net::read_chunk_until_cancellable`,
`net::write_chunk_until`,
`net::write_chunk_until_cancellable`,
`net::write_chunks_until`, and
`net::write_chunks_until_cancellable` infer both `net` and `time` because
the adapter-owned accept, read, or write attempt observes a `Deadline` or
`CancelToken`.
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
`net::close_stream` records fixture-backed adapter-owned stream cleanup and
returns `()`. `net::close_listener` records fixture-backed adapter-owned
listener cleanup and returns `()`; after that close, `net::accept`,
`net::accept_or_end`, `net::accept_until`, and
`net::accept_until_cancellable` fail as runtime transport failures instead of
reporting clean end, deadline expiry, or cancellation.
Connected and accepted streams expose endpoint text through
`net::stream_local_addr` and `net::stream_peer_addr`, and use the same
read, write, write-side shutdown, and close helpers. Forced connection
failure remains a runtime transport failure.
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
without replacing the read clean-end path, `net::close_stream` closes the
owned stream, and a following optional or deadline-aware accept can observe
clean listener end.
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
task::spawn(job: fn() -> T effects [concurrency]) -> Task<T> effects [concurrency]
task::spawn<T>(job: fn() -> T effects [concurrency]) -> Task<T> effects [concurrency]
task::spawn_with(job: fn(C) -> T effects [concurrency], context: C) -> Task<T> effects [concurrency]
task::spawn_with<T>(job: fn(C) -> T effects [concurrency], context: C) -> Task<T> effects [concurrency]
task::spawn_with<T, C>(job: fn(C) -> T effects [concurrency], context: C) -> Task<T> effects [concurrency]
task::join(task: Task<T>) -> Result<T, JoinError> effects [concurrency]
task::cancel(task: Task<T>) -> () effects [concurrency]
```

`task::spawn` starts a zero-argument callable in a concurrent task and returns
its task handle. `task::spawn_with` starts a one-argument callable with one
ordinary source context value. The optional first explicit type argument fixes
the task item type, and the optional second explicit type argument fixes the
context parameter type for `task::spawn_with<T, C>(job, context)`. The context
value may be an anonymous record or any existing named type accepted by the
handler. Numbered multi-argument task spawn calls are not standard task
operations; callers carry multiple values through one context argument.
Arguments are frozen before crossing into the task, and the result value is
frozen before it crosses back through the task handle.
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

## Prelude Helpers

Every user module is checked with an implicit standard `prelude` import.
Prelude helper exports are ordinary pure helper calls for name-resolution
purposes: bare helper names resolve when no local declaration shadows them and
no written import creates an ambiguity, and `prelude::name` selects the
standard helper explicitly. The helpers are registered in the standard symbol
table as pure compatibility helpers or source-backed pure helpers, so a name
must be present in that table before the prelude signature adapter assigns its
compiler-known type. They do not infer effects. No `List`/`Vec` conversion
helpers are part of this public helper set; names such as `list_to_vec` or
`vec_to_list` resolve only when user declarations put them in scope.

### Standard Byte ADTs

```veln
type StreamInput
	Chunk(bytes: ByteChunk)
	End
end

type AcceptOutcome
	AcceptStream(stream: NetStream)
	AcceptEnd
	AcceptDeadlineExpired
	AcceptCancelled
end

type StreamReadOutcome
	ReadChunk(bytes: ByteChunk)
	ReadEnd
	ReadDeadlineExpired
	ReadCancelled
end

type StreamWriteOutcome
	WriteCompleted
	WriteDeadlineExpired
	WriteCancelled
end

type DecodeError
	DecodeError(id: String, offset: ByteOffset, field_path: String)
	DecodeErrorWithReason(id: String, offset: ByteOffset, field_path: String, reason: String)
end

type DecodeReadiness
	NeedBytes(count: ByteCount)
	NeedEnd
end

type DecodeStep<T>
	Decoded(value: T, consumed: ByteCount)
	NeedMore(readiness: DecodeReadiness)
	Invalid(error: DecodeError)
end

type EncodeError
	EncodeError(id: String, field_path: String, reason: String)
end

type EncodeStep<TState>
	Encoded(chunks: List<ByteChunk>)
	Partial(chunks: List<ByteChunk>, produced: ByteCount, state: TState)
	Invalid(error: EncodeError)
end
```

`StreamInput` is the source-visible incremental input event type. `Chunk`
carries an ordinary immutable `ByteChunk`, including empty chunks, and `End`
is the explicit end-of-stream event.

Source code can model retained pending input by appending incoming
`StreamInput.Chunk` bytes into an immutable `ByteChunk`, checking a
source-owned `ByteCount` limit, taking bounded `ByteView` prefixes for parsing,
dropping consumed bytes, and tracking the next absolute `ByteOffset`
separately for diagnostics. Source code can also collect outgoing immutable
`ByteChunk` values in `List<ByteChunk>` protocol action values without a
separate output chunk type.

`DecodeStep<T>` is the source-visible incremental decode transition type.
`Decoded` carries the decoded value and consumed `ByteCount`, `NeedMore`
carries `DecodeReadiness`, and `Invalid` carries a structured `DecodeError`.
`NeedBytes` names the minimum buffered byte count required before retrying, and
`NeedEnd` represents decoders that need an explicit end-of-stream event.

`EncodeStep<TState>` is the source-visible incremental encode transition
type. `Encoded` carries the complete immutable output chunks, `Partial`
carries committed output chunks, their produced `ByteCount`, and the encoder
state that owns the remaining work, and `Invalid` carries a structured
`EncodeError`. `EncodeError` carries a stable id, source-visible field path,
and representation-failure reason.

`ByteView` is the source-visible bounded immutable byte view. Programs create
checked views with `byte_view(chunk, offset, count)` and inspect the bounded
bytes with the byte-view helper functions; the runtime does not expose a
source-visible borrow lifetime or zero-copy layout guarantee.

### Helper Signatures

```veln
byte(value: Int) -> Result<Byte, String>
byte_to_int(value: Byte) -> Int
flag8_is_set(flags: Flag8, index: Int) -> Result<Bool, String>
flag8_set(flags: Flag8, index: Int) -> Result<Flag8, String>
flag8_bits(flags: Flag8) -> Int
flag8_from_bits(bits: Int) -> Result<Flag8, String>
flag16be_is_set(flags: Flag16be, index: Int) -> Result<Bool, String>
flag16be_set(flags: Flag16be, index: Int) -> Result<Flag16be, String>
flag16be_bits(flags: Flag16be) -> Int
flag16be_from_bits(bits: Int) -> Result<Flag16be, String>
flag16le_is_set(flags: Flag16le, index: Int) -> Result<Bool, String>
flag16le_set(flags: Flag16le, index: Int) -> Result<Flag16le, String>
flag16le_bits(flags: Flag16le) -> Int
flag16le_from_bits(bits: Int) -> Result<Flag16le, String>
flag24be_is_set(flags: Flag24be, index: Int) -> Result<Bool, String>
flag24be_set(flags: Flag24be, index: Int) -> Result<Flag24be, String>
flag24be_bits(flags: Flag24be) -> Int
flag24be_from_bits(bits: Int) -> Result<Flag24be, String>
flag24le_is_set(flags: Flag24le, index: Int) -> Result<Bool, String>
flag24le_set(flags: Flag24le, index: Int) -> Result<Flag24le, String>
flag24le_bits(flags: Flag24le) -> Int
flag24le_from_bits(bits: Int) -> Result<Flag24le, String>
flag32be_is_set(flags: Flag32be, index: Int) -> Result<Bool, String>
flag32be_set(flags: Flag32be, index: Int) -> Result<Flag32be, String>
flag32be_bits(flags: Flag32be) -> Int
flag32be_from_bits(bits: Int) -> Result<Flag32be, String>
flag32le_is_set(flags: Flag32le, index: Int) -> Result<Bool, String>
flag32le_set(flags: Flag32le, index: Int) -> Result<Flag32le, String>
flag32le_bits(flags: Flag32le) -> Int
flag32le_from_bits(bits: Int) -> Result<Flag32le, String>
flag40be_is_set(flags: Flag40be, index: Int) -> Result<Bool, String>
flag40be_set(flags: Flag40be, index: Int) -> Result<Flag40be, String>
flag40be_bits(flags: Flag40be) -> Int
flag40be_from_bits(bits: Int) -> Result<Flag40be, String>
flag40le_is_set(flags: Flag40le, index: Int) -> Result<Bool, String>
flag40le_set(flags: Flag40le, index: Int) -> Result<Flag40le, String>
flag40le_bits(flags: Flag40le) -> Int
flag40le_from_bits(bits: Int) -> Result<Flag40le, String>
flag48be_is_set(flags: Flag48be, index: Int) -> Result<Bool, String>
flag48be_set(flags: Flag48be, index: Int) -> Result<Flag48be, String>
flag48be_bits(flags: Flag48be) -> Int
flag48be_from_bits(bits: Int) -> Result<Flag48be, String>
flag48le_is_set(flags: Flag48le, index: Int) -> Result<Bool, String>
flag48le_set(flags: Flag48le, index: Int) -> Result<Flag48le, String>
flag48le_bits(flags: Flag48le) -> Int
flag48le_from_bits(bits: Int) -> Result<Flag48le, String>
flag56be_is_set(flags: Flag56be, index: Int) -> Result<Bool, String>
flag56be_set(flags: Flag56be, index: Int) -> Result<Flag56be, String>
flag56be_bits(flags: Flag56be) -> Int
flag56be_from_bits(bits: Int) -> Result<Flag56be, String>
flag56le_is_set(flags: Flag56le, index: Int) -> Result<Bool, String>
flag56le_set(flags: Flag56le, index: Int) -> Result<Flag56le, String>
flag56le_bits(flags: Flag56le) -> Int
flag56le_from_bits(bits: Int) -> Result<Flag56le, String>
flag64be_is_set(flags: Flag64be, index: Int) -> Result<Bool, String>
flag64be_set(flags: Flag64be, index: Int) -> Result<Flag64be, String>
flag64be_bits(flags: Flag64be) -> Int
flag64be_from_bits(bits: Int) -> Result<Flag64be, String>
flag64le_is_set(flags: Flag64le, index: Int) -> Result<Bool, String>
flag64le_set(flags: Flag64le, index: Int) -> Result<Flag64le, String>
flag64le_bits(flags: Flag64le) -> Int
flag64le_from_bits(bits: Int) -> Result<Flag64le, String>
byte_chunk(bytes: Vec<Byte>) -> ByteChunk
byte_chunk_count(chunk: ByteChunk) -> ByteCount
byte_append(left: ByteChunk, right: ByteChunk) -> ByteChunk
byte_chunk_from_hex(text: String) -> Result<ByteChunk, String>
byte_chunk_to_visible_ascii_string(chunk: ByteChunk) -> Result<String, String>
byte_chunk_from_visible_ascii_string(text: String) -> Result<ByteChunk, String>
byte_take(chunk: ByteChunk, count: ByteCount) -> Result<ByteChunk, String>
byte_drop(chunk: ByteChunk, count: ByteCount) -> Result<ByteChunk, String>
byte_view(chunk: ByteChunk, offset: ByteOffset, count: ByteCount) -> Result<ByteView, String>
byte_view_to_chunk(view: ByteView) -> ByteChunk
byte_view_count(view: ByteView) -> ByteCount
byte_view_take(view: ByteView, count: ByteCount) -> Result<ByteView, String>
byte_view_drop(view: ByteView, count: ByteCount) -> Result<ByteView, String>
byte_view_slice(view: ByteView, offset: ByteCount, count: ByteCount) -> Result<ByteView, String>
byte_chunks_empty() -> List<ByteChunk>
byte_chunks_one(chunk: ByteChunk) -> List<ByteChunk>
byte_chunks_append(left: List<ByteChunk>, right: List<ByteChunk>) -> List<ByteChunk>
byte_chunks_produce(chunks: List<ByteChunk>, budget: ByteCount) -> {chunks: List<ByteChunk>, produced: ByteCount, remaining: List<ByteChunk>}
byte_read_u8_be(view: ByteView) -> Result<Int, String>
byte_expect_fixed_u8_be(view: ByteView, expected: Int, schema_name: String, field_name: String) -> Result<Int, String>
byte_decode_http2_frame(view: ByteView) -> Result<{length: Int, kind: Int, flags: Int, stream_id: Int, payload: ByteView}, String>
byte_decode_schema_width_sample(view: ByteView) -> Result<{short_value: Int, wide_value: Int}, String>
byte_decode_schema_validation_sample(view: ByteView) -> Result<{length: Int, padding_length: Int}, String>
http2_protocol_closed_with_pending(offset: Int, pending_count: Int, active_continuation: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_protocol_partial_preface(offset: Int, pending_count: Int, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_protocol_invalid_preface(offset: Int, expected_byte: Int, actual_byte: Int, matched_count: Int, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_protocol_continuation_expected(offset: Int, actual_kind: Int, actual_stream: Int, expected_stream: Int, started_kind: Int, started_offset: Int, active_continuation: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_protocol_invalid_frame_kind(offset: Int, actual_kind: Int, stream_id: Int, expected_kind: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_protocol_invalid_stream_id(offset: Int, frame_kind: Int, stream_id: Int, required_domain: String, endpoint_role: String, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_protocol_invalid_payload_length(offset: Int, frame_kind: Int, stream_id: Int, observed_length: Int, expected_length: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_protocol_invalid_window_update_increment(offset: Int, stream_id: Int, observed_increment: Int, accepted_min_increment: Int, accepted_max_increment: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_protocol_invalid_request_header_list(offset: Int, frame_kind: Int, stream_id: Int, failed_header_fact: String, header_name: String, decoded_header_names: String, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_protocol_invalid_response_header_list(offset: Int, frame_kind: Int, stream_id: Int, failed_header_fact: String, header_name: String, decoded_header_names: String, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_protocol_content_length_mismatch(offset: Int, frame_kind: Int, stream_id: Int, expected_length: Int, observed_length: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_protocol_invalid_priority_dependency(offset: Int, stream_id: Int, dependency_stream_id: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_protocol_stream_after_goaway(offset: Int, stream_id: Int, last_stream_id: Int, shutdown_state: String, endpoint_role: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_peer_limit_frame_size_exceeded(offset: Int, observed_length: Int, allowed_length: Int, frame_kind: Int, stream_id: Int, receive_limit_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_peer_limit_header_list_size_exceeded(offset: Int, observed_size: Int, allowed_size: Int, frame_kind: Int, stream_id: Int, receive_limit_provenance: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_peer_limit_header_table_size_exceeded(offset: Int, observed_size: Int, allowed_size: Int, frame_kind: Int, stream_id: Int, receive_limit_provenance: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_peer_limit_flow_control_window_exceeded(offset: Int, observed_length: Int, allowed_window_credit: Int, frame_kind: Int, stream_id: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_peer_limit_concurrent_streams_exceeded(offset: Int, stream_id: Int, attempted_count: Int, allowed_count: Int, endpoint_role: String, active_state: String, receive_limit_provenance: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2_peer_limit_settings_value_out_of_range(offset: Int, setting_identifier: Int, setting_name: String, observed_value: Int, accepted_min_value: Int, accepted_max_value: Int, peer_limit_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
hpack_fixture_table_size_update_malformed(offset: Int, observed_size: Int, observed_first_byte: Int, expected_fixture: String, codec_module: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
hpack_fixture_table_size_update_not_at_start(offset: Int, observed_size: Int, observed_first_byte: Int, observed_update_size: Int, frame_kind: Int, stream_id: Int, active_state: String, expected_fixture: String, codec_module: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
hpack_fixture_table_size_update_trailing_bytes(offset: Int, observed_size: Int, observed_first_byte: Int, observed_update_size: Int, frame_kind: Int, stream_id: Int, active_state: String, expected_fixture: String, codec_module: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
byte_read_u16_be(view: ByteView) -> Result<Int, String>
byte_read_u24_be(view: ByteView) -> Result<Int, String>
byte_read_u31_be(view: ByteView) -> Result<Int, String>
byte_read_u32_be(view: ByteView) -> Result<Int, String>
byte_read_u40_be(view: ByteView) -> Result<Int, String>
byte_read_u48_be(view: ByteView) -> Result<Int, String>
byte_read_u56_be(view: ByteView) -> Result<Int, String>
byte_read_u64_be(view: ByteView) -> Result<Int, String>
byte_read_u16_le(view: ByteView) -> Result<Int, String>
byte_read_u24_le(view: ByteView) -> Result<Int, String>
byte_read_u31_le(view: ByteView) -> Result<Int, String>
byte_read_u32_le(view: ByteView) -> Result<Int, String>
byte_read_u40_le(view: ByteView) -> Result<Int, String>
byte_read_u48_le(view: ByteView) -> Result<Int, String>
byte_read_u56_le(view: ByteView) -> Result<Int, String>
byte_read_u64_le(view: ByteView) -> Result<Int, String>
byte_write_u8_be(value: Int) -> Result<ByteChunk, String>
byte_write_u16_be(value: Int) -> Result<ByteChunk, String>
byte_write_u24_be(value: Int) -> Result<ByteChunk, String>
byte_write_u31_be(value: Int) -> Result<ByteChunk, String>
byte_write_u32_be(value: Int) -> Result<ByteChunk, String>
byte_write_u40_be(value: Int) -> Result<ByteChunk, String>
byte_write_u48_be(value: Int) -> Result<ByteChunk, String>
byte_write_u56_be(value: Int) -> Result<ByteChunk, String>
byte_write_u64_be(value: Int) -> Result<ByteChunk, String>
byte_write_u16_le(value: Int) -> Result<ByteChunk, String>
byte_write_u24_le(value: Int) -> Result<ByteChunk, String>
byte_write_u31_le(value: Int) -> Result<ByteChunk, String>
byte_write_u32_le(value: Int) -> Result<ByteChunk, String>
byte_write_u40_le(value: Int) -> Result<ByteChunk, String>
byte_write_u48_le(value: Int) -> Result<ByteChunk, String>
byte_write_u56_le(value: Int) -> Result<ByteChunk, String>
byte_write_u64_le(value: Int) -> Result<ByteChunk, String>
byte_count(value: Int) -> Result<ByteCount, String>
byte_count_to_int(value: ByteCount) -> Int
byte_offset(value: Int) -> Result<ByteOffset, String>
byte_offset_to_int(value: ByteOffset) -> Int
vec_len(items: Vec<A>) -> Int
vec_is_empty(items: Vec<A>) -> Bool
vec_push(items: Vec<A>, value: A) -> Vec<A>
vec_concat(left: Vec<A>, right: Vec<A>) -> Vec<A>
vec_map(items: Vec<A>, f: fn(A) -> B) -> Vec<B>
vec_filter(items: Vec<A>, f: fn(A) -> Bool) -> Vec<A>
vec_fold(items: Vec<A>, initial: B, f: fn(B, A) -> B) -> B
vec_try_map(items: Vec<A>, f: fn(A) -> Result<B, E>) -> Result<Vec<B>, E>
vec_try_map_with(context: C, items: Vec<A>, f: fn(C, A) -> Result<B, E>) -> Result<Vec<B>, E>
list_nil() -> List<A>
list_cons(head: A, tail: List<A>) -> List<A>
list_is_empty(items: List<A>) -> Bool
list_fold(items: List<A>, initial: B, f: fn(B, A) -> B) -> B
list_reverse(items: List<A>) -> List<A>
list_map(items: List<A>, f: fn(A) -> B) -> List<B>
list_filter(items: List<A>, f: fn(A) -> Bool) -> List<A>
list_try_map(items: List<A>, f: fn(A) -> Result<B, E>) -> Result<List<B>, E>
dict_get(dict: Dict<K, V>, key: K) -> Option<V>
dict_contains(dict: Dict<K, V>, key: K) -> Bool
dict_insert(dict: Dict<K, V>, key: K, value: V) -> Dict<K, V>
dict_remove(dict: Dict<K, V>, key: K) -> Dict<K, V>
dict_map(dict: Dict<K, V>, f: fn(K, V) -> A) -> Dict<K, A>
dict_map_with(context: C, dict: Dict<K, V>, f: fn(C, K, V) -> A) -> Dict<K, A>
dict_filter(dict: Dict<K, V>, f: fn(K, V) -> Bool) -> Dict<K, V>
dict_filter_with(context: C, dict: Dict<K, V>, f: fn(C, K, V) -> Bool) -> Dict<K, V>
dict_fold(dict: Dict<K, V>, initial: A, f: fn(A, K, V) -> A) -> A
dict_fold_with(context: C, dict: Dict<K, V>, initial: A, f: fn(C, A, K, V) -> A) -> A
dict_try_map(dict: Dict<K, V>, f: fn(K, V) -> Result<A, E>) -> Result<Dict<K, A>, E>
dict_try_map_with(context: C, dict: Dict<K, V>, f: fn(C, K, V) -> Result<A, E>) -> Result<Dict<K, A>, E>
option_map(value: Option<A>, f: fn(A) -> B) -> Option<B>
option_and_then(value: Option<A>, f: fn(A) -> Option<B>) -> Option<B>
option_unwrap_or(value: Option<A>, fallback: A) -> A
result_map(value: Result<A, E>, f: fn(A) -> B) -> Result<B, E>
result_map_err(value: Result<A, E>, f: fn(E) -> F) -> Result<A, F>
result_and_then(value: Result<A, E>, f: fn(A) -> Result<B, E>) -> Result<B, E>
string_split_once(text: String, separator: String) -> Option<{left: String, right: String}>
string_parse_int(text: String) -> Result<Int, String>
int_to_string(value: Int) -> String
```

The generated schema helper signatures in this list are compatibility and
runtime adapter signatures. Source code should apply schemas through explicit
schema `decode` and `encode` expressions or through ordinary wrapper
functions that call those expressions.

### Value Semantics

Container update helpers return new frozen values and do not mutate their input
containers in place. `vec_len` returns the number of items in the input vec.
`vec_concat` returns a vec containing the left input's items followed by the
right input's items. `vec_is_empty` returns whether a vec contains no items.
`dict_contains` returns true when `dict_get` would return `Some` for the same
dictionary and key, and false when `dict_get` would return `None`.
`dict_map`, `dict_map_with`, `dict_filter`, `dict_filter_with`, `dict_fold`,
`dict_fold_with`, `dict_try_map`, and `dict_try_map_with` visit dictionary
entries in insertion order and pass each key and value to the callback. The
`_with` aliases pass the unchanged context value as the first callback
argument. `dict_map` and `dict_map_with` preserve keys and map values.
`dict_filter` and `dict_filter_with` preserve entries whose callback returns
true. `dict_fold` and `dict_fold_with` thread the accumulator through each
entry. `dict_try_map` and `dict_try_map_with` stop calling their callback
after the first `Err`; otherwise they return `Ok` containing the mapped frozen
dictionary.
`vec_try_map` evaluates items in source order, stops at the first `Err`, and
otherwise returns `Ok` containing the mapped frozen vec in source order.
`vec_try_map_with` follows the same traversal and passes the unchanged context
value as the first callback argument. `vec_map`, `vec_filter`, and `vec_fold`
also visit vec items in source order.
`list_nil` and `list_cons` construct `List` values equivalent to `Nil` and
`Cons`. `list_is_empty` returns true for `Nil` and false for `Cons`.
`list_reverse` returns a list with the input items in reverse order.
`list_map`, `list_filter`, `list_fold`, and `list_try_map` visit list items in
source order. `list_try_map` stops at the first `Err`; otherwise it returns
`Ok` containing the mapped list in source order. List traversal helpers are
implemented without relying on source-level tail-recursion syntax. Public JVM
helper calls for large list traversals do not consume one host stack frame per
list element, and this remains runtime support rather than a general
tail-call optimization guarantee.

`string_split_once` splits at the first occurrence of `separator`, returning
`None` when the separator is absent. `string_parse_int` accepts the backend
integer spelling and returns the original input string in `Err` when parsing
fails. `int_to_string` renders an integer for display and string composition.

`byte(value)` accepts integers from `0` through `255` and returns `Err(String)`
for values outside that range. `flag8_is_set` and `flag8_set` read and set
`Flag8` bit indexes `0` through `7`; `flag16be_is_set` and `flag16be_set`
read and set `Flag16be` bit indexes `0` through `15`; `flag16le_is_set` and
`flag16le_set` read and set `Flag16le` bit indexes `0` through `15`;
`flag24be_is_set` and `flag24be_set` read and set `Flag24be` bit indexes `0`
through `23`; `flag24le_is_set` and `flag24le_set` read and set `Flag24le`
bit indexes `0` through `23`; `flag32be_is_set` and `flag32be_set` read and
set `Flag32be` bit indexes `0` through `31`; `flag32le_is_set` and
`flag32le_set` read and set `Flag32le` bit indexes `0` through `31`;
`flag40be_is_set` and `flag40be_set` read and set `Flag40be` bit indexes `0`
through `39`; `flag40le_is_set` and `flag40le_set` read and set `Flag40le`
bit indexes `0` through `39`;
`flag48be_is_set` and `flag48be_set` read and set `Flag48be` bit indexes `0`
through `47`; `flag48le_is_set` and `flag48le_set` read and set `Flag48le`
bit indexes `0` through `47`;
`flag56be_is_set` and `flag56be_set` read and set `Flag56be` bit indexes `0`
through `55`; `flag56le_is_set` and `flag56le_set` read and set `Flag56le`
bit indexes `0` through `55`;
`flag64be_is_set` and `flag64be_set` read and set `Flag64be` bit indexes `0`
through `63`; `flag64le_is_set` and `flag64le_set` read and set `Flag64le`
bit indexes `0` through `63`. Each
checked flag helper returns `Err(String)` for indexes outside its supported
range instead of masking or wrapping. `flag8_bits`, `flag16be_bits`,
`flag16le_bits`, `flag24be_bits`, `flag24le_bits`, `flag32be_bits`,
`flag32le_bits`, `flag40be_bits`, `flag40le_bits`, `flag48be_bits`,
`flag48le_bits`, `flag56be_bits`, `flag56le_bits`, `flag64be_bits`, and
`flag64le_bits` expose the wrapped integer bits.
`flag8_from_bits`, `flag16be_from_bits`, `flag16le_from_bits`,
`flag24be_from_bits`, `flag24le_from_bits`, `flag32be_from_bits`,
`flag32le_from_bits`, `flag40be_from_bits`, `flag40le_from_bits`,
`flag48be_from_bits`, `flag48le_from_bits`, `flag56be_from_bits`,
`flag56le_from_bits`, `flag64be_from_bits`, and `flag64le_from_bits` return
`Err(String)` for integers outside the one-byte, two-byte, three-byte,
four-byte, five-byte, six-byte, seven-byte, or eight-byte flag range before
an invalid flag value reaches generated schema
encode helpers.
`byte_chunk(bytes)` returns an immutable owned
chunk containing the supplied bytes. `byte_chunk_count(chunk)` returns the
chunk length as `ByteCount`. `byte_append(left, right)` returns a new chunk
with the left bytes followed by the right bytes. `byte_chunk_from_hex(text)`
accepts only ASCII hex byte pairs with ASCII whitespace between complete bytes
and returns `Ok(ByteChunk)` for the decoded bytes. It returns `Err(String)`
with `fixture.hex.invalid_character` for non-hex text, prefixes, underscores,
comments, separators, non-ASCII characters, or whitespace inside a byte pair,
and `fixture.hex.odd_length` for a dangling final nibble. The error text
includes the decoded byte offset and the high or low nibble position. When the
error propagates out of `run --json`, the runtime result details expose the
fixture text span, decoded `ByteOffset`, nibble position, and nearby context.
`byte_chunk_to_visible_ascii_string(chunk)` returns `Ok(String)` when every
byte in the chunk is visible ASCII from `0x21` through `0x7e`, preserving byte
order as characters, and returns `Err(String)` for any byte outside that range.
`byte_chunk_from_visible_ascii_string(text)` returns `Ok(ByteChunk)` when every
character is visible ASCII from `0x21` through `0x7e`, preserving character
order as bytes, and returns `Err(String)` for any character outside that range.
`byte_take(chunk, count)` and `byte_drop(chunk, count)` return `Ok(ByteChunk)`
when `count` is within the chunk length, and `Err(String)` when the count is
outside that chunk.
`byte_view(chunk, offset, count)` returns a bounded immutable `ByteView` when
the non-negative offset and count describe a range within the chunk, and
returns `Err(String)` when the range exceeds the chunk length. `byte_view` and
byte reads report negative direct-constructor payloads with the same
non-negative offset and count error strings as the construction helpers.
`byte_view_to_chunk(view)` materializes exactly the bounded bytes as an
immutable owned `ByteChunk`.
`byte_view_count(view)` returns the view length as `ByteCount`.
`byte_view_take(view, count)`, `byte_view_drop(view, count)`, and
`byte_view_slice(view, offset, count)` derive bounded immutable views within
the supplied view and return `Err(String)` when the requested local range
exceeds the view length. These helpers let source code represent pending input
as bounded `ByteView` values while keeping the absolute `ByteOffset` carried by
the view. `byte_chunks_empty()`, `byte_chunks_one(chunk)`, and
`byte_chunks_append(left, right)` construct and combine `List<ByteChunk>`
values for outgoing chunks without introducing an output-only byte type.
`byte_chunks_produce(chunks, budget)` returns the prefix chunks that fit within
the supplied `ByteCount` budget, the produced byte count, and the remaining
suffix. It preserves chunk order, never splits a `ByteChunk`, returns no
produced chunks for a zero budget, and leaves the remaining suffix unchanged
when the first chunk does not fit.
The fixed-width unsigned big-endian and little-endian read helpers read from
the start of the view and return `Err(String)` when the view is too short.
The `u31` and `u64` reads also return `Err(String)` when the decoded value
would exceed the source-visible `Int` maximum for the helper width. The
source-visible `u56` helpers read and write the same seven-byte big-endian or
little-endian representation as `UInt56be` and `UInt56le`, accept values in
the `0..72057594037927935` range, and reject shorter views or unrepresentable
write values with `Err(String)`.
The
`byte_expect_fixed_u8_be` helper reads one byte and returns
`schema.fixed_field_mismatch` diagnostic details when the actual byte differs
from the expected fixed byte for the supplied schema and field names. The
`byte_decode_schema_width_sample` remains as compatibility coverage for the
narrow executable schema slice for `UInt16be` and `UInt32be`: it reads both
fields from a `ByteView`, returns ordinary `Int` values, and reports schema
truncation with field-path byte diagnostic details. Compatibility binary
schema helper lowering also accepts `UInt16le`,
`UInt24le`, `UInt31le`, `UInt32le`, `UInt40le`, `UInt48le`, `UInt56le`, and
`UInt64le` as little-endian unsigned fields. `UInt40be` uses the matching
five-byte big-endian representation, `UInt48be` uses the matching six-byte
big-endian representation, `UInt56be` uses the matching seven-byte
big-endian representation, and `UInt64be` uses the matching eight-byte
big-endian representation. Those
fields decode to ordinary `Int` when representable and encode with the same
representability boundaries as their matching unsigned widths.
Source `format binary` schema declarations whose fields
all use implemented exact-width unsigned primitives retain compatibility
`byte_decode_<schema>` helpers in their declaring module. Those helpers decode
fields in schema order, check supported field-local `where` predicates after
the owning field is decoded, project field-local fixed equality predicates
through `Err(RuntimeDiagnostic(...))` with `schema.fixed_field_mismatch` when
the decoded value differs, return ordinary `Int` values when validation
passes. Projection from the schema-local record into a domain shape is ordinary
source code at the explicit operation or compatibility helper boundary.
They report `schema.validation_failed` with field path, predicate, decoded
values, and structured byte preview fields when validation fails. The same
eligible schema declarations also retain compatibility
`byte_decode_step_<schema>` helpers that accept `ByteView` plus `ByteOffset`
and return `DecodeStep<T>` with
`Decoded(value, consumed)` for a complete buffered value or
`NeedMore(NeedBytes(count))` for an open view that is too short to decide. The
exact-width, supported reserved-bit, length-bounded `ByteView`, closed
dispatch, extension dispatch, and eligible nested dispatch payload encode
slices retain compatibility
`byte_encode_<schema>` helpers for eligible binary schemas whose
source-visible fields are exact-width unsigned primitives, supported
byte-aligned `ReservedBits(width, value)` fields, the supported
`ReservedBits(1, 0)` before `UInt31be` layout, the supported
`ReservedBits(2, 0)` and `ReservedBits(9, 0)` before `UInt8` byte-prefix
layouts, supported
prefix `ReservedBits(width, value)` plus `UIntN` layouts whose widths
complete one, two, three, or four big-endian bytes, supported `UIntN` plus
reserved suffix layouts whose widths complete one, two, three, four, five, six,
seven, or eight big-endian bytes, supported visible `UInt8` plus
non-byte-aligned multi-byte `ReservedBits(width, value)` suffix layouts that
fit in one three-byte through eight-byte big-endian storage unit with low
padding, supported `UIntN` plus middle
`ReservedBits(width, value)` plus `UIntN` layouts whose widths complete one,
two, three, or four big-endian bytes, including the narrow two-byte
interleaved middle layout with a sub-byte visible `UIntN`, a reserved field,
`UInt8`, and a final sub-byte visible `UIntN`, supported
`ReservedBits(width, value)` plus two visible sub-byte or byte-width `UIntN`
prefix groups whose widths complete one, two, three, or four big-endian bytes,
supported consecutive non-byte-aligned
`UIntN` and `ReservedBits(width, value)` groups whose widths complete one,
two, three, four, five, six, seven, or eight big-endian bytes,
bounded `Repeat(count_field, Payload)` fields whose count names an earlier
visible exact-width field and whose payload is an exact-width unsigned
primitive, an eligible nested binary schema, or
`ByteView(length_field)` whose length names an earlier visible exact-width
field,
length-bounded
`ByteView(length_field)` fields whose
length names an earlier visible exact-width field,
`ByteView(left_length - right_length)` fields whose operands both name earlier
visible exact-width fields, closed dispatch fields, or extension-tolerant
dispatch fields with earlier visible exact-width tag and length fields. Dispatch
payload cases may be exact-width visible primitive payloads, including
lowercase `uint...` and `flag...` spelling, or eligible nested binary schema
payloads named as earlier same-module binary schemas or public imported binary
schemas through written `use` paths. Those helpers
accept a schema-local visible
record, using ordinary `Int` fields for visible primitives, `ByteView` fields
for length-bounded payloads, `List<ByteView>` fields for repeated bounded
byte-view payloads, and `SchemaDispatchPayload<T>` for extension dispatch
payload fields, and return `Result<ByteChunk, EncodeError>` with field-order
output using each primitive's declared byte order, each supplied byte view's
bounded bytes, or a structured encode error. The
supported reserved-bit encode layout omits byte-aligned
`ReservedBits(width, value)` fields from the value record and writes their
declared fixed values. It also omits `ReservedBits(1, 0)` from the value
record when it immediately precedes `UInt31be`; it omits
`ReservedBits(2, 0)` or `ReservedBits(9, 0)` from the value record when it
immediately precedes `UInt8` and writes the declared reserved prefix, visible
byte, and zero low padding bits when present in one two-byte bitstream slice;
supported packed
prefix layouts omit the reserved field and write the declared high bits with
the visible low-bit record field in the shared storage unit. Supported suffix
layouts omit the reserved field and write the visible high-bit record field
with the declared low reserved bits in the shared storage unit. Supported
middle layouts omit the reserved field and write both adjacent visible record
fields around the declared reserved bits in the shared storage unit. Supported
consecutive non-byte-aligned `UIntN` and `ReservedBits(width, value)` groups
omit every reserved field and write visible and declared reserved values in
declaration order in the shared storage unit. The closed
dispatch encode layout selects the payload width from the earlier visible tag
field and reports `schema.dispatch_unknown_tag` when no
case matches. The extension dispatch encode layout writes `Known` selected
payloads, preserves matching unknown raw payload bytes, reports
`schema.dispatch_mismatch` for tag or variant disagreements, and reports
`schema.dispatch_length_mismatch` when the explicit length field differs from
the emitted payload byte count. The fixed-width unsigned big-endian and
little-endian read helpers return `Ok(Int)` when the bounded `ByteView`
contains enough bytes and the decoded unsigned value fits the helper width;
they return `Err(String)` for short views or values larger than the helper
width can represent, such as the 31-bit maximum check. The fixed-width
unsigned big-endian and little-endian write helpers return `Ok(ByteChunk)` for
values in range and `Err(String)` for negative values or values larger than
the helper width can encode.
`byte_count(value)` and `byte_offset(value)` accept non-negative integers.
The `*_to_int` helpers expose the stored integer value for ordinary source
logic and display.

### Source-Backed Boundary

The implemented standard symbol table has this current pure-helper split.
This table records compiler-known runtime symbols, including compatibility
helpers, rather than the public schema application surface.

- source-backed pure helpers: `byte`, `byte_to_int`, `byte_chunk`,
  `byte_chunk_count`, `byte_append`, `byte_chunk_from_hex`,
  `byte_chunk_to_visible_ascii_string`,
  `byte_chunk_from_visible_ascii_string`, `byte_take`, `byte_drop`,
  `byte_view`, `byte_view_to_chunk`, `byte_view_count`,
  `byte_view_take`, `byte_view_drop`, `byte_view_slice`,
  `byte_chunks_empty`, `byte_chunks_one`, `byte_chunks_append`,
  `byte_chunks_produce`,
  `byte_read_u8_be`,
  `byte_expect_fixed_u8_be`,
  `byte_decode_http2_frame`, `byte_decode_schema_width_sample`,
  `byte_decode_schema_validation_sample`,
  `http2_protocol_closed_with_pending`,
  `http2_protocol_partial_preface`,
  `http2_protocol_invalid_preface`,
  `http2_protocol_continuation_expected`,
  `http2_protocol_invalid_frame_kind`,
  `http2_protocol_invalid_stream_id`,
  `http2_protocol_invalid_payload_length`,
  `http2_protocol_invalid_window_update_increment`,
  `http2_protocol_invalid_request_header_list`,
  `http2_protocol_invalid_response_header_list`,
  `http2_protocol_content_length_mismatch`,
  `http2_protocol_invalid_priority_dependency`,
  `http2_protocol_stream_after_goaway`,
  `http2_peer_limit_frame_size_exceeded`,
  `http2_peer_limit_header_list_size_exceeded`,
  `http2_peer_limit_header_table_size_exceeded`,
  `http2_peer_limit_flow_control_window_exceeded`,
  `http2_peer_limit_concurrent_streams_exceeded`,
  `http2_peer_limit_settings_value_out_of_range`, `byte_read_u16_be`,
  `byte_read_u24_be`, `byte_read_u31_be`, `byte_read_u32_be`,
  `byte_read_u40_be`, `byte_read_u48_be`, `byte_read_u56_be`,
  `byte_read_u64_be`,
  `byte_read_u16_le`, `byte_read_u24_le`, `byte_read_u31_le`,
  `byte_read_u32_le`, `byte_read_u40_le`, `byte_read_u48_le`,
  `byte_read_u56_le`, `byte_read_u64_le`, `byte_write_u8_be`,
  `byte_write_u16_be`, `byte_write_u24_be`, `byte_write_u31_be`,
  `byte_write_u32_be`, `byte_write_u40_be`, `byte_write_u48_be`,
  `byte_write_u56_be`, `byte_write_u64_be`, `byte_write_u16_le`,
  `byte_write_u24_le`, `byte_write_u31_le`, `byte_write_u32_le`,
  `byte_write_u40_le`, `byte_write_u48_le`, `byte_write_u56_le`,
  `byte_write_u64_le`, `byte_count`, `byte_count_to_int`, `byte_offset`,
  `byte_offset_to_int`,
  `vec_len`, `vec_is_empty`, `vec_push`, `vec_concat`, `vec_map`,
  `vec_filter`, `vec_fold`, `vec_try_map`, `vec_try_map_with`,
  `list_nil`, `list_cons`, `list_is_empty`, `list_fold`, `list_reverse`,
  `list_map`, `list_filter`, `list_try_map`, `dict_get`, `dict_contains`,
  `dict_insert`, `dict_remove`, `dict_map`, `dict_map_with`, `dict_filter`,
  `dict_filter_with`, `dict_fold`, `dict_fold_with`, `dict_try_map`,
  `dict_try_map_with`, `option_map`, `option_and_then`, `option_unwrap_or`,
  `result_map`, `result_map_err`, `result_and_then`, `string_split_once`,
  `string_parse_int`, and `int_to_string`
- descriptor-only pure helpers: none

The `http2_protocol_invalid_payload_length` helper is source-backed and
returns `Result<(), RuntimeDiagnostic>`, matching the source-visible
invalid-payload-length detail used by the HTTP/2 protocol-core fixed
payload-length examples, including `WINDOW_UPDATE`.

This empty descriptor-only pure-helper list is the implemented completion
condition for the self-hosting prelude helper migration. Every compiler-known
pure helper in this split is source-backed, while float operator compatibility
descriptors remain outside the migration candidate pool.

The source-visible `RuntimeDiagnosticDetail` constructor set includes
`RuntimeValueDiagnostic(...)` for projecting generated binary schema encode
value diagnostics from source-visible `RuntimeDiagnostic(...)` error values,
`RuntimeHttp2ProtocolClosedWithPendingDiagnostic(...)` for projecting
`http2.protocol.closed_with_pending` failures,
`RuntimeHttp2ProtocolPartialPrefaceDiagnostic(...)` for projecting
`http2.protocol.partial_preface` failures,
`RuntimeHttp2ProtocolInvalidPrefaceDiagnostic(...)` for projecting
`http2.protocol.invalid_preface` failures,
`RuntimeHttp2ProtocolContinuationExpectedDiagnostic(...)` for projecting
`http2.protocol.continuation_expected` failures,
`RuntimeHttp2PeerLimitHeaderListSizeDiagnostic(...)` for projecting
`http2.peer_limit.header_list_size_exceeded` failures,
`RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic(...)` for projecting
`http2.peer_limit.header_table_size_exceeded` failures,
`RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic(...)` for projecting
`http2.peer_limit.concurrent_streams_exceeded` failures,
`RuntimeHttp2PeerLimitSettingsValueDiagnostic(...)` for projecting
`http2.peer_limit.settings_value_out_of_range` failures,
`RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(...)` for projecting
`http2.protocol.invalid_frame_kind` failures from returned diagnostic values,
`RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(...)` for projecting
`http2.protocol.invalid_stream_id` failures from returned diagnostic values,
`RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic(...)` for projecting
`http2.protocol.invalid_data_padding` failures from returned diagnostic
values,
`RuntimeHttp2PeerLimitFlowControlWindowDiagnostic(...)` for projecting
`http2.peer_limit.flow_control_window_exceeded` failures from returned
diagnostic values,
`RuntimeHttp2ProtocolContentLengthMismatchDiagnostic(...)` for projecting
`http2.protocol.content_length_mismatch` failures from returned diagnostic
values,
`RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(...)` for projecting
`http2.protocol.invalid_request_header_list` failures from returned diagnostic
values,
`RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(...)` for projecting
`http2.protocol.invalid_response_header_list` failures from returned
diagnostic values,
`RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic(...)` for
projecting `http2.protocol.invalid_window_update_increment` failures from
returned diagnostic values,
`RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic(...)` for projecting
`http2.protocol.unexpected_settings_ack` failures from returned diagnostic
values,
`RuntimeHttp2ProtocolPriorityDependencyDiagnostic(...)` for projecting
`http2.protocol.invalid_priority_dependency` failures from returned diagnostic
values, and
`RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic(...)` for projecting
`http2.protocol.stream_after_goaway` failures from returned diagnostic values.
`http2_protocol_closed_with_pending(...)`,
`http2_protocol_partial_preface(...)`,
`http2_protocol_invalid_preface(...)`,
`http2_protocol_continuation_expected(...)`,
`http2_protocol_invalid_frame_kind(...)`,
`http2_protocol_invalid_stream_id(...)`,
`http2_protocol_invalid_data_padding(...)`,
`http2_protocol_invalid_window_update_increment(...)`,
`http2_protocol_content_length_mismatch(...)`,
`http2_protocol_unexpected_settings_ack(...)`,
`http2_protocol_invalid_priority_dependency(...)`,
`http2_protocol_stream_after_goaway(...)`,
`http2_peer_limit_frame_size_exceeded(...)`,
`http2_peer_limit_header_list_size_exceeded(...)`,
`http2_peer_limit_header_table_size_exceeded(...)`,
`http2_peer_limit_flow_control_window_exceeded(...)`,
`http2_peer_limit_concurrent_streams_exceeded(...)`, and
`http2_peer_limit_settings_value_out_of_range(...)`,
`http2_protocol_invalid_request_header_list(...)`, and
`http2_protocol_invalid_response_header_list(...)` return these payloads
directly as `Result<(), RuntimeDiagnostic>`.

Use [Helper Signatures](#helper-signatures) for the implemented signature of
each helper and [Value Semantics](#value-semantics) for behavior. The
descriptor-only list above is the implemented candidate pool for proposal work
that moves one already specified pure helper into embedded source. When it is
empty, there is no current pure-helper target for this proposal route.

Source-backed status is descriptor metadata as described in
[Compiler-Known Descriptor Table](#compiler-known-descriptor-table). The
embedded source is ordinary Veln source in the `prelude` module, with one
descriptor entry per exported helper entry point. The source metadata records
the repository-relative standard library path and entry function name used for
checking the embedded helper source. The current checker still uses the
descriptor-backed signature adapter, and the JVM backend still lowers each
helper through the existing prelude runtime operation, so diagnostics stay
anchored on user call sites rather than the embedded standard library source.
Source-backed helpers are declared in `prelude` as public functions and may use
other existing helpers. Embedded helper source may call compiler-known prelude
runtime operations through the reserved `prelude_builtin` module, such as
`prelude_builtin::vec_fold(items, initial, f)`, to avoid spelling a runtime
operation like an ordinary recursive call to the helper being defined.
`vec_len` delegates to `prelude_builtin::vec_len` so the runtime can use the
host vec size directly. The vec traversal helpers use
`prelude_builtin::vec_fold`, and vec append support uses
`prelude_builtin::vec_push`; their step helpers are implementation details, and
this source placement does not expose or stabilize a public vec
representation. Byte hex fixture decoding and byte slice helpers delegate
through `prelude_builtin::byte_chunk_from_hex`,
`prelude_builtin::byte_chunk_to_visible_ascii_string`,
`prelude_builtin::byte_take`, and `prelude_builtin::byte_drop` because text
decoding, visible ASCII conversion, and bounded slicing
currently cross the runtime container boundary. The `vec_fold` entry is
declared in the shared `prelude`
source and delegates to `prelude_builtin::vec_fold`. The list
helpers use the descriptor-backed `List<A>` constructors and pattern coverage;
their private step helpers are ordinary support source and do not expose a
public list representation beyond `Nil` and `Cons`. The dict helpers keep
using the existing prelude runtime operation through
`prelude_builtin::dict_get`, `prelude_builtin::dict_insert`, and
`prelude_builtin::dict_remove`; their public bare names remain source-backed
descriptor entry points, and `dict_contains` derives its result from the
builtin get operation. Private support functions such as
`vec_try_map_with_step` and `list_try_map_step` are ordinary support source and
are not separate prelude descriptors.

### Compiler-Support Source

The embedded `compiler_support` source contains
`load_source_text(path: Path) -> Result<String, FsError> effects [fs]`. It is
not a prelude helper. It is a small compiler-support subsystem used to exercise
Veln source checking and JVM execution through `fs::read_to_string`.

### Diagnostics And Tests

When `vec_map` receives a callback whose return type is `Result`, the checker
reports the ordinary callback type mismatch and adds a repair hint to use
`vec_try_map` for fallible traversal.

The language specification does not promise asymptotic complexity, allocation
counts, representation identity, structural sharing, hashing, or tree-balancing
behavior for these helpers. Those are implementation details until a concrete
container representation is specified. Tests should assert value semantics,
source-order traversal, `Result` short-circuiting, diagnostics, and effect
behavior rather than timings or allocation counts.
