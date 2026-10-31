---
role: specification
authority: normative
update-when: Veln effect labels, compiler-known call signatures, nominal handlers, or effect-boundary behavior changes.
specification-coverage: usage=#representative-usage; behavior=#effect-labels; limits=#limits
---

# Effects

## Representative usage

The following handler discharges the nominal `Ask` effect for the handled body:

```veln
effect Ask
	value() -> Int
end

handler ask(offset: Int) for Ask
	value() => offset + 1
end

fn compute() -> Int effects [Ask]
	perform Ask::value()
end

test handled() -> Result<(), String>
	let observed = handle compute() with ask(41)
	if observed == 42
		Ok(())
	else
		Err("handler returned unexpected value")
	end
end
```

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
row-polymorphic function is called, argument compatibility determines one
substitution for `E` across all nested occurrences in the call signature. The
[type compatibility rules](types.md#compatibility-and-limits) define the
structural constraint directions, least-set solution, retained rigid enclosing
row tails, and rejected bounds. A call with no lower bound uses an empty
substitution, including when the row occurs only in the result type. An
accepted call instantiates both its result and its effects, then unions the row
with concrete effects written beside `...E`. An accepted call keeps that
instantiated row when another argument is a `perform` expression; the performed
effect also contributes through the ordinary expression-effect rule. A rejected
call exposes neither the tentative result row nor its effects, including through
a local function binding, a rejected constructor payload, or an incompatible
value propagated with `?`. Public boundary diagnostics for a fully concrete
accepted call use the concrete instantiated effects.

Source modules may also declare nominal operation effects with `effect Name`
or `pub effect Name`. Each operation declares ordinary parameter types and one
result type. Effect-operation parameter and result annotations and handler
context parameter annotations use the same variant-refinement validity rules
as other type positions. An ineligible base reports its base failure. A union
whose successfully instantiated alternatives have different resolved ADT
identities or canonical generic arguments reports `type.variant_union_base`.
If one operation parameter or result has either failure, every parameter and
the result of that operation is unknown to
`perform` expressions and handler clauses. If one handler context parameter
has either failure, every context parameter is unknown in handler clauses and
at handler application sites. These boundaries preserve independently
provable declaration diagnostics and suppress mismatches derived only from a
sibling annotation, as specified by [Types](types.md#compatibility-and-limits)
and the
[diagnostic contract](diagnostics-json.md#diagnostic-families). The effect name
is owned by its module. Same-module declarations may use the short name in
`effects [...]` lists and `perform` expressions. Imported declarations may use
an exact written module path or an eligible local import qualifier. For
example, both `transport::DuplexStream` after `use transport` and
`wire::DuplexStream` after `use transport as wire` retain the
`std::transport::DuplexStream` identity. An explicitly aliased import supplies
no bare effect candidate or implicit `transport` qualifier. The
[name-resolution contract](name-resolution.md#explicit-import-aliases) owns
exact-path precedence, qualifier collisions, and import quarantine. Function
type annotations use the same effect-list spelling. Unknown nominal effects in
function type annotations are rejected at the effect path inside the
annotation.
A function type nested in a module `let` annotation receives the same label
validation. A module value has no effect row binder, so a row tail in that
annotation reports `effect.row_unbound`; an unknown label reports
`effect.unknown`.
Nominal labels in declaration effect lists and function type annotations
resolve through the imports owned by that declaration's package and module.
Same-spelled modules in different packages may therefore bind the same local
qualifier to distinct nominal effects. Parameter, result, and local-binding
annotations retain the selected package identity through checking and
lowering, independent of source loading order.
Nominal effect paths that use an import with an invalid module path segment or
invalid explicit alias follow that name-resolution quarantine boundary. One
visible public effect export can suppress the derivative `effect.unknown`
diagnostic, but private targets, wrong-kind targets, missing targets, and
missing modules remain independently reported.

`perform E::operation(arguments)` resolves `E` as a nominal effect and resolves
`operation` in that effect declaration. The checker validates the argument
types against the declared operation parameter types. The expression type is
the declared operation result type. The expression contributes the nominal
effect to the containing function's inferred effect set. A public function must
declare each non-empty inferred effect; a pure public declaration omits its
effect clause. Private functions use the existing private effect inference
rule. Duplicate operation declarations are reported at the duplicate
operation name span and include the first declaration as related context. An
unknown operation is reported at the operation name span. An unknown performed
effect is reported at the performed effect path. Runnable `veln run` and
`veln test` entry boundaries reject a retained user-defined effect before JVM
execution, including effects inferred for a private run entry. Exported
library functions may retain user-defined effects in their signatures.
When a `.test.veln` companion writes an explicit `use` for its exact target,
the same qualified target path can name a private target nominal effect in
`perform`, declaration effect lists, function type annotation effect lists,
companion-local handler `for` targets, and declared handler effect lists.
The permission is exact and
non-transitive. Bare names, missing imports, wrong-target companions,
`_test.veln` integration modules, and external packages do not receive this
private effect access.

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
Handler paths may use the same exact written paths and explicit import
qualifiers as effect paths. A selected imported handler keeps its declaration
module as the lookup scope for private functions called by its operation
clauses; the local qualifier does not become runtime identity. An import selects
effects and handlers from both the target module and target package. An effect
or handler in a same-spelled module owned by another package is not a candidate
and cannot replace a missing declaration in the selected package. Effect
inference, handler discharge, and private handler dependency inference retain
that selected package identity; a namesake effect or private dependency from
another package does not enter the inferred effect set. A path that
uses an import with an invalid module path segment or invalid explicit alias
follows the same name-resolution quarantine boundary: one visible public
handler export can suppress the derivative `handler.unknown` diagnostic, but
private targets, wrong-kind targets, missing targets, and missing modules remain
independently reported.

The checker evaluates the effect set of a handle expression as the union of
context argument effects, body effects with the handled nominal effect removed,
and the handler declaration effects. Handler context arguments are evaluated
left to right before the body. Handling is deep for calls made during the body,
and nested handlers for the same operation shadow outer handlers until the
nested body finishes. Handler state is lexical to the current task. Task
creation expressions expose their job effect rows at the call expression, so a
lexical handler around the task creation expression can discharge a handled
nominal job effect before the runnable entry boundary is checked.

Postfix `?` propagation and contract or runtime failure restore handler frames
as they leave handled
expressions. A cleanup registered inside a handled expression runs before that
expression's handler frame is restored. If cleanup registered outside an inner
handled expression performs the same effect, the cleanup observes the outer
handler. Cleanup registrations and handler frames unwind in reverse lexical
nesting order. If restoring a handler frame fails while `?` is propagating an
ordinary `Err` result, the cleanup failure interrupts that result propagation;
enclosing cleanup registrations and handler frames still unwind. Cleanup
failure precedence and task-cancellation behavior are specified by
[execution.md](execution.md#runtime-readiness-and-host-boundaries).

Effects in a `begin` body and in every deferred block contribute to the
enclosing function, test, or handler operation clause. A cleanup-region
boundary does not discharge or hide an effect. A `begin` used as a handler
operation clause expression therefore contributes both its direct effects and
the effects of calls made from its body and deferred blocks to that handler's
retained effect set.

The exported standard `observe` module declares the public nominal
`observe::Observe` effect. Handling a body with `observe::noop()` removes that
effect without adding another effect. Handling it with
`observe::recording(recording)` removes `observe::Observe` and adds `time` and
`concurrency` from the handler clauses. Handling it with
`observe::production(policy)` removes `observe::Observe` and adds
`concurrency`; constructing the policy and reading its health snapshot also
require `concurrency`. Handling with `observe::json_lines(exporter)` removes
`observe::Observe` and adds `time` and `concurrency`. Its host write boundary
uses real configured destinations unless a scoped
`host_effects::ObservationExporter` handler is installed by test support; no
environment variable or system property selects that substitution.
`observe::new_buffered_json_lines` adds `concurrency` and creates a worker that
inherits the active observation-exporter adapter. Direct buffered admission
adds `time` and `concurrency`; buffered health and normal shutdown add
`concurrency`. The inherited adapter remains attached to the worker after the
constructing handler scope exits. The
`trace::in_span<effect E>` helper
retains `observe::Observe` together with the effect row `E` of its callback
until an enclosing handler discharges the observation effect. The
[standard-library observability specification](standard-library-observability.md)
defines the operations, recording behavior, and scoped span lifecycle.

The exported standard `transport` module declares this public nominal effect:

```veln
pub effect DuplexStream
	read_chunk() -> Result<Option<ByteChunk>, net::NetError>
	write_chunks(chunks: List<ByteChunk>) -> Result<(), net::NetError>
end
```

The exported `transport::net` module declares
`net_stream(stream: net::Stream)` as a public lexical handler for
`transport::DuplexStream`. Handling a body with `transport::net::net_stream`
removes `std::transport::DuplexStream` from the inferred effect set and adds
`std::net::IO` from the handler clauses. A public function
that performs a duplex-stream operation without a handler must declare the
duplex-stream effect. A public function that wraps that body with the
`net_stream` handler must declare `net::IO` and does not retain the handled
duplex-stream effect.
The equivalent `transport::network::net_stream` spelling avoids the `net`
import-alias collision when one source file also imports `net` from `std` to
name `net::Stream`, `net::NetError`, or `net::IO`.
`http2::connection::drive_server` and `http2::connection::drive_client` expose
only `std::transport::DuplexStream`; handling either driver with `net_stream`
therefore replaces that nominal effect with `std::net::IO` and does not expose
HTTP/2-internal effects.
`http2::connection::drive_server_application<effect E>` exposes
`std::transport::DuplexStream` plus the effect row of its application
callback. The callback type is
`fn(Http2ApplicationEvent) -> Result<List<Http2ApplicationAction>, String>
effects [...E]`, and the driver requires
`[std::transport::DuplexStream, ...E]`. Handling that driver with
`transport::net::net_stream` removes only the duplex-stream effect and leaves
callback effects such as `db` on the handled expression.

`http2::connection::serve_tcp<effect E>` composes this driver with listener,
stream, and task ownership. Its public effect set is
`[std::net::IO, concurrency, ...E]`. Each spawned connection task inherits the
caller's active `std::net::IO` handler. Its internal
`transport::network::net_stream` handler removes only
`std::transport::DuplexStream`; it does not remove the inherited network
effect or a callback effect. The [HTTP/2 specification](http2.md#connection-drivers-and-services)
defines the service result and cleanup behavior.

`http2::connection::request_endpoint_sequence<effect E>` has the same public
effect set, `[std::net::IO, concurrency, ...E]`, for the TCP client. Its
spawned request tasks inherit the active `std::net::IO` handler. Their
`transport::network::net_stream` handlers adapt owned streams without
installing a host provider or consuming callback effects.
`http2::connection::request_endpoint_sequence_tcp<effect E>` installs
`net::system()` around that service. Its public effect set is
`[net, time, concurrency, ...E]`; the handler removes `std::net::IO`, exposes
the two host effects, and leaves task and callback effects unchanged.

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
the helper omitted its own `effects` clause. Pure public functions and tests
omit the clause; an explicit empty declaration is rejected. Function-body effect inference
follows direct bare function calls and `use` alias qualified function calls
until a fixed point. Public function aliases carry the referenced function's
signature and effects. Calls through a local binding with a function type infer
the effects written in that function type.
Calls to the exported `std::net` facade through an explicit `net` import
contribute the declared nominal `net::IO` effect. The compiler does not supply
a source-less `net::` fallback.

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

`fs::path(text: String) -> Path` constructs a path value from host path text
without reading or writing the file system. It is pure and does not infer
`fs`. Host-invalid path text can fail at runtime. Relative paths are resolved
by the file-system operation against the current working directory.

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

Source must import the exported network module with `use net from "std"`.
Network operations retain the nominal `net::IO` effect until a lexical handler
discharges it. The exported `net::system()` handler discharges `net::IO` and
retains the coarse host `net` and `time` effects. Its frame owns the resources
created within the handled scope and closes remaining resources before the
frame is restored. Expected host failures become typed `net::NetError` values;
failed `listen` and `connect` results retain command-diagnostic provenance
through ordinary result propagation. The complete source API and its limits
are specified by
[Standard Library Networking](standard-library-networking.md).

The compiler-known time boundary remains source-less:

```veln
time::monotonic_ms() -> Int effects [time]
time::wall_time() -> WallTime effects [time]
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

These calls infer `time`. `time::wall_time` returns a normalized structural
`WallTime` record whose `unix_seconds` counts UTC seconds relative to the Unix
epoch and whose `nanosecond` is in `0..999999999`; host clock resolution and
clock correction remain observable. `Deadline` and `time::monotonic_ms` own
duration and ordering behavior. Cancellation owners can create observer tokens
with `time::cancel_token_from`; only the owner can cancel such a token through
`time::cancel_owned`. Direct tokens remain cancellable through `time::cancel`.
The cancellable wait outcome reports `WaitCompleted`, `WaitDeadlineExpired`,
or `WaitCancelled` as an ordinary value.

## Scoped host adapters

The standard `host_effects` module exposes nominal `Network`, `Clock`,
`TaskJoin`, and `ObservationExporter` effects for substituting or observing
host boundaries. Install an ordinary Veln lexical handler around the calls
that need controlled results:

```veln
use host_effects from "std"

handler fake_clock() for host_effects::Clock
	request(operation, milliseconds) => Ok(42)
	wall_time() => Ok({ unix_seconds: 0, nanosecond: 0 })
end

fn controlled_reading() -> Int effects [time]
	handle time::monotonic_ms() with fake_clock()
end
```

The clock handler supplies the monotonic millisecond reading in this example.
The underlying call still contributes the coarse `time` effect. Host adapter
injection does not remove coarse `net` or `time` requirements from callers.
Without a handler, clock operations use the real host clock and waits, and
socket operations use real host sockets, and chunk-only receive/send operations
use host standard input/output. Environment variables and system
properties do not select fake results.

`TaskJoin::waiting()` is called immediately before `task::join` waits for a
task that is incomplete at the join boundary. An installed handler may record
or synchronize that boundary; its result does not replace the join outcome.
No operation is performed when the task is already complete, and without a
handler the join waits with no observation callback. This hook makes task
waiting observable without changing production task scheduling or selecting
test behavior through process state.

`ObservationExporter::write(destination, line, diagnostic)` receives the
resolved `stderr` or `file:<path>` target, the exact newline-terminated
payload, and whether the payload is a diagnostic summary. Its Boolean result
reports whether the destination accepted the write. A record that the JSON
Lines export boundary rejects during validation does not perform this
operation.
`ObservationExporter::identity(kind, sequence)` supplies the identity used for
an exported span. These operations let scoped Veln test support capture writes
and provide deterministic identities. Without a handler, the runtime uses the
real destination and host-generated identities described by
[Execution Boundary](execution.md#runtime-readiness-and-host-boundaries). The
public handler, record, and failure contracts belong to
[Standard-library Observability](standard-library-observability.md#synchronous-json-lines-export).

`Clock::request(operation, milliseconds)` returns `Result<Int, String>`.
The bridge uses these requests:

| Operation | Input | Meaning of `Ok(value)` |
| --- | --- | --- |
| `now` | `0` | Monotonic milliseconds. |
| `sleep` | Requested timeout milliseconds. | The wait completed; the integer is ignored. |
| `wait` | Remaining deadline milliseconds. | The wait completed; the integer is ignored. |
| `cancelled` | `0` | Nonzero requests token cancellation. |
| `wait_cancellable` | Remaining deadline milliseconds. | `0` completes, `1` reports deadline expiry, and `2` reports cancellation. |

An `Err(message)` becomes a runtime failure. Clock handlers must keep their
readings and deadline calculations in the same clock domain.

`Clock::wall_time()` returns one atomic
`Result<{unix_seconds: Int, nanosecond: Int}, String>` reading. The bridge
normalizes the returned nanosecond field into the public range and adjusts the
seconds field with floor arithmetic. If that adjustment would move
`unix_seconds` outside the `Int` range, the read fails at runtime with
`wall-clock seconds exceed Int range`; it does not return a wrapped instant.
The operation's `Err(message)` also becomes a runtime failure. This operation
is independent of the monotonic domain used by `Clock::request`.

`Network::request(operation, subject, bytes, remaining_ms, cancelled)` receives
the network operation name and its address or adapter-owned resource identity.
Write operations supply the outgoing bytes. `remaining_ms` is `-1` without a
deadline and otherwise the remaining duration; `cancelled` reports the supplied
token state. The reply is an ordinary record with these fields:

| Fields | Bridge interpretation |
| --- | --- |
| `status` | `ok` supplies a successful result; `end`, `deadline`, and `cancelled` select the corresponding accept or read outcome, or deadline/cancellation write outcome. `error` raises a transport failure at the direct bridge. When `net::system()` consumes the request, it translates that failure into the operation's typed `NetError` result. |
| `value` | Created resource identity or endpoint/state query text. Boolean queries use `true` or `false`. |
| `bytes` | Received bytes for a successful read. |
| `resolved_endpoints` | Ordered resolver results. Each item names `tcp4` or `tcp6` and one numeric host address. The system handler preserves this order and removes only exact network-and-address duplicates. Other operations use an empty list. |
| `bytes_committed` | Exact prefix length committed by a system-handler write, including deadline, cancellation, shutdown, and other failure replies. Other operations use zero. |
| `local`, `peer` | Endpoint text; empty text means unavailable failure context. |
| `category`, `phase`, `cause` | Failure category, lifecycle phase, and related cause. |
| `input_committed`, `output_committed`, `ownership_committed` | Failure commit facts: `0` means false, `1` means true, and a negative value means unknown. |

Handlers supply the complete typed reply record declared by the standard
module. `bytes_committed` is independent of the three commit facts and never
changes their tri-state interpretation. A failed `shutdown_read` consumes
`input_committed`, a failed `shutdown_write` consumes `output_committed`, and a
failed listener or stream close consumes `ownership_committed`. A false fact
preserves the affected prior state and leaves it retryable. A true fact commits
the closed state. An unknown fact makes the affected resource or half
unavailable while retaining the cleanup obligation and permitting an explicit
retry. The resource-state transitions, including retries after an unknown fact,
are specified by
[standard-library-networking.md](standard-library-networking.md#network-operation-boundary).

A scoped network handler receives `resolve_tcp`, `resolve_tcp4`, or
`resolve_tcp6` for system-handler resolution; the request carries the remaining
deadline and cancellation state supplied to `connect_with`. Therefore
deterministic resolution does not consult ambient
DNS, and it observes the same controls as the later connection attempts. This
adapter boundary does not expose host sockets or remove caller responsibility
for resource lifecycle. The supported operation and outcome combinations are
those of the network calls described above.

Nested handlers shadow the corresponding outer handler during their body.
Leaving a handled body restores the outer handler, including after failure.
Spawned tasks inherit the handlers active when they are created. Independent
executions must construct independent fake state. Network handles retain the
adapter that created them, so operations through those handles continue to use
that adapter rather than changing resource identity when a scope ends.

Repository deterministic tests keep fake behavior in separate Veln support
sources and pass those sources with the fixture to the compiler. The default
runtime contains the host adapter bridge; fake input sequences and failure
plans belong to the injected Veln handler.

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
or test that calls one of them must declare those non-empty effects in its
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
cancellable channel-first adapter maps that result into an ordinary source
route outcome so cancellation is a visible adapter completion state.
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
task::join_error_is_cancelled(error: JoinError) -> Bool effects [concurrency]
task::cancel(task: Task<T>) -> () effects [concurrency]
```

`task::spawn` starts a zero-argument callable in a concurrent task and returns
its task handle. `task::spawn_with` starts a one-argument callable with one
ordinary source context value. A pure job substitutes the empty effect set for
`E`, so the task creation expression infers only `concurrency`. An effectful
job substitutes its concrete effect set for `E`, so the task creation
expression infers `concurrency` plus each job effect once. A job that handles
`transport::DuplexStream` with `transport::net::net_stream(stream)` substitutes
the handled expression's remaining effects plus `std::net::IO`; the task creation
expression does not retain `transport::DuplexStream` after handler
replacement. Pure, effectful, and context jobs retain these declared effect
rows.

The optional first explicit type argument fixes the task item type, and the
optional second explicit type argument fixes the context parameter type for
`task::spawn_with<T, C>(job, context)`. The context value may be an anonymous
record or any existing named type accepted by the handler. Numbered
multi-argument task spawn calls are not standard task operations; callers
carry multiple values through one context argument. Arguments are frozen
before crossing into the task, and the result value is frozen before it
crosses back through the task handle.
`task::join` waits for completion and returns `Ok(value)` when the task returns
normally. Cancellation, interruption, contract failure, and another
non-runtime task failure return `Err(JoinError)`. An ordinary runtime failure
from the task crosses the join boundary as that runtime failure instead of
becoming a join error. Failure message text does not change this
classification.
`task::join_error_is_cancelled` returns `true` only for the classified
cancelled join outcome and does not change the error or task. `task::cancel`
requests cancellation by interrupting the task and returns `()`. The first
call that observes an incomplete task claims cancellation atomically.
Concurrent or later calls do not send another interruption, including while
the task is running deferred cleanup. A call that observes an already completed
task has no effect. Cancellation is cooperative at the JVM runtime boundary,
and [execution](execution.md#runtime-readiness-and-host-boundaries) specifies
cleanup completion before `task::join` reports cancellation.

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

## Limits

Effect inference is conservative at source boundaries: compiler-known calls and
reachable helpers contribute only the coarse labels listed above, while unknown
or user-defined effects remain in exported signatures and are rejected at
`veln run` or `veln test` entry points. Runtime transport, deadline,
cancellation, channel, and task failures remain runtime outcomes; they do not
become schema or codec diagnostics. The public effect surface does not promise
TLS, ALPN, or an application protocol, and source-visible handles must follow
the close and cancellation ownership rules described in the network and time
sections.

## References

- Effect inference and handlers: `crates/veln-sema/src/effects.rs` and
  `crates/veln-sema/src/effect_rows.rs`.
- Compiler-known signatures: `crates/veln-sema/src/standard_symbols/`.
- Wall-clock record type boundaries:
  `examples/specification/check/transport-wall-clock-record-type/` and
  `examples/specification/check/transport-wall-clock-name-shadowing/`.
- Qualified, aliased, and user-effect wall-clock type positions:
  `examples/specification/check/transport-wall-clock-type-resolution/`.
- Explicit-import effect and handler identity:
  `examples/specification/check/explicit-import-alias-carriers/`,
  `examples/specification/run/explicit-import-alias-handler/`, and
  `examples/specification/run/explicit-import-alias-handler-identity/`.
