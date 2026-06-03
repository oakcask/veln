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
`process`, and `concurrency` from compiler-known calls. The other labels are
reserved coarse-grained public boundary labels for source compatibility.

## Compiler-Known Descriptor Table

Semantic analysis owns a standard symbol table for compiler-known library
symbols. The table records the source-visible module, name, symbol kind, effect
labels, lowering identity, and stability class for the descriptor-backed
subset.

The current descriptor-backed subset covers stdio effect metadata,
concurrency effect metadata, minimal `fs` and `process` intrinsics, pure
prelude helper admission, and source provenance for source-backed pure helpers.
Type adapters and most runtime lowering still use their existing specialized
implementations.

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
channel::select_timeout(left: Receiver<T>, right: Receiver<T>, timeout_ms: Int) -> Option<{index: Int, value: T}> effects [concurrency]
channel::select_result(left: Receiver<T>, right: Receiver<T>) -> Result<Option<{index: Int, value: T}>, SelectError> effects [concurrency]
channel::select_priority_result(left: Receiver<T>, right: Receiver<T>) -> Result<Option<{index: Int, value: T}>, SelectError> effects [concurrency]
channel::select_timeout_result(left: Receiver<T>, right: Receiver<T>, timeout_ms: Int) -> Result<Option<{index: Int, value: T}>, SelectError> effects [concurrency]
channel::close(tx: Sender<T>) -> () effects [concurrency]
```

Direct calls to these functions infer the `concurrency` effect. A public
function or test that calls one of them must declare `concurrency` in its
`effects [...]` list.

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
`channel::select_timeout(left, right, timeout_ms)` has the same receiver and
return typing as `channel::select`, plus an `Int` millisecond timeout. It
returns `None` when the timeout elapses before a value is selected. Negative
timeouts wait without a timeout.
`channel::select_result`, `channel::select_priority_result`, and
`channel::select_timeout_result` use the same selection rules as their
non-result counterparts, but return `Ok(Some(selected))` for a selected value,
`Ok(None)` for closed or timed-out selection, and `Err(SelectError)` when
cooperative cancellation interrupts the waiting selection.
`channel::close` closes the sender endpoint, wakes waiting receivers, and
returns `()`.

The checker also recognizes these task-operation call targets:

```veln
task::spawn(job: fn() -> T effects [concurrency]) -> Task<T> effects [concurrency]
task::spawn<T>(job: fn() -> T effects [concurrency]) -> Task<T> effects [concurrency]
task::join(task: Task<T>) -> Result<T, JoinError> effects [concurrency]
task::cancel(task: Task<T>) -> () effects [concurrency]
```

`task::spawn` starts a zero-argument callable in a concurrent task and returns
its task handle. The result value is frozen before it crosses back through the
task handle. `task::join` waits for completion and returns `Ok(value)` when the
task returns normally, or `Err(JoinError)` when the task is interrupted,
cancelled, or fails at runtime. `task::cancel` requests cancellation by
interrupting the task and returns `()`. Cancellation is cooperative at the JVM
runtime boundary.

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

### Helper Signatures

```veln
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

### Value Semantics

Container update helpers return new frozen values and do not mutate their input
containers in place. `vec_len` returns the number of items in the input vec.
`vec_concat` returns a vec containing the left input's items followed by the
right input's items. `vec_is_empty` returns whether a vec contains no items.
`dict_contains` returns true when `dict_get` would return `Some` for the same
dictionary and key, and false when `dict_get` would return `None`.
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

### Source-Backed Boundary

The implemented standard symbol table has this current pure-helper split:

- source-backed pure helpers: `vec_len`, `vec_is_empty`, `vec_push`,
  `vec_concat`, `vec_map`, `vec_filter`, `vec_fold`, `vec_try_map`,
  `vec_try_map_with`, `list_nil`, `list_cons`, `list_is_empty`, `list_fold`,
  `list_reverse`, `list_map`, `list_filter`, `list_try_map`, `dict_get`,
  `dict_contains`, `dict_insert`, `dict_remove`, `option_map`,
  `option_and_then`, `option_unwrap_or`, `result_map`, `result_map_err`,
  `result_and_then`, `string_split_once`, `string_parse_int`, and
  `int_to_string`
- descriptor-only pure helpers: none

This empty descriptor-only pure-helper list is the implemented completion
condition for the self-hosting prelude helper migration. Every compiler-known
pure helper in this split is source-backed, while float operator compatibility
descriptors remain outside the migration candidate pool.

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
representation. The `vec_fold` entry is declared in the shared `prelude`
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
