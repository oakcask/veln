# Discussion Result: Runtime Value Freeze Boundary

Status: implemented

## Picked Question

- What runtime ownership or freeze rule is needed so immutable lists,
  dictionaries, records, strings, and other values can cross task or channel
  boundaries without dirty reads or representation corruption?

## Decision

Require a runtime freeze boundary for shareable Veln values.

Ordinary first-slice Veln values are share-safe once they are visible to user
code. This includes primitives, strings, records, lists, dictionaries,
`Option`, and `Result` values whose contained values are also ordinary Veln
values. Such values may be sent through channels, captured by spawned tasks,
and read concurrently by multiple Veln tasks.

The implementation may use mutable builders, temporary arrays, hash tables, or
other writable construction state internally. That state must not be reachable
from user code or cross a task boundary until it has been frozen. After a value
is frozen, every representation reachable from that value is transitively
immutable from the runtime's point of view: no task may update it in place,
including as an optimization for a later functional container operation.

Channel send and task spawn must safely publish the frozen value to the
receiving task. The runtime must provide the host-memory-ordering guarantee
needed for writes performed during construction and freeze to be visible to
the receiver before the value is read.

Backend-owned resources are not ordinary share-safe Veln values. Foreign
objects, file handles, sockets, native pointers, and other host resources need
explicit send-safety metadata or wrapper types before they can cross task or
channel boundaries.

## Rationale

Source-level immutability is not sufficient by itself. A runtime that stores a
list as a mutable array or a dictionary as a mutable hash table can still break
concurrent programs if it shares that representation between tasks and later
updates it in place. The user would see an immutable value, but another task
could observe partially initialized state, stale state, or a container whose
internal invariants were being changed by a producer.

The freeze boundary keeps the language contract simple while giving the
implementation room to be efficient. Builders and mutation are allowed before
publication, but the value handed to user code is a frozen snapshot. Functional
updates produce a new frozen value and may use structural sharing only through
nodes that will never be mutated again.

This rule also fits the channel-first concurrency decision. Channels can move
or share ordinary values without exposing ownership annotations in user code,
while still giving the runtime a clear safety check: only frozen ordinary Veln
values, or explicitly send-safe host values, may be published across tasks.

## First-Slice Rules

- Ordinary Veln values are transitively share-safe after freeze.
- Values reachable from user code are frozen values, not mutable builders.
- A mutable builder or temporary container is task-local runtime state and
  cannot be sent through a channel or captured by a spawned task.
- Freezing a value makes every representation reachable from that value
  immutable for the rest of its lifetime.
- Functional container updates return a new frozen value. They must not mutate
  an existing frozen value in place.
- Structural sharing is allowed only when the shared nodes are frozen and will
  not be mutated by any later operation.
- Channel send and task spawn safely publish frozen values according to the
  selected backend's memory model.
- The checker and runtime should treat backend-owned resources separately from
  ordinary Veln values. Such resources require explicit send-safety metadata
  before they cross task or channel boundaries.

## Open Details

The exact internal representation remains open. A first JVM implementation can
start with copy-on-update immutable wrappers around arrays and maps, then move
to persistent vector or hash-array mapped trie representations later if
performance requires it.

The decision does not require a user-visible ownership system. A future design
may add linear, affine, or unique-value capabilities for specialized resources,
but ordinary immutable Veln values should remain easy to send and share.

The concrete shape of send-safety metadata for host resources remains a
foreign-function and standard-library design topic.

## Consequence

Veln can promise that ordinary immutable values are safe to pass between
parallel tasks without exposing Rust-style ownership in the first slice. The
runtime gets a precise implementation rule: mutation is allowed while building
a value, but once a value is frozen and reachable from user code or a channel,
it must be safely published and never mutated in place.
