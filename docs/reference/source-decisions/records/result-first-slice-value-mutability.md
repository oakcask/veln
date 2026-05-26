# Discussion Result: First-Slice Value Mutability

Status: implemented

## Picked Question

- Should first-slice bindings and container values be mutable, and what memory
  management contract does that imply?

## Decision

Use immutable value semantics in the first slice.

`let` bindings are not assignable after introduction. Records, lists,
dictionaries, `Option`, `Result`, strings, and other first-slice aggregate
values are immutable from the user's point of view. Container operations follow
the functional style: they return a new value that represents the updated
container state instead of mutating the existing value in place.

Implementations may use structural sharing, copy-on-write, arenas, reference
counting, tracing GC, or a host runtime's collector, but those strategies are
not observable language behavior. The language contract is automatic memory
management for values that are no longer reachable from running code, not a
specific collector algorithm or collection timing.

The first slice should not expose object identity, pointer equality, mutable
references, finalizers, weak references, destructors, or user-controlled
allocation and deallocation.

## Rationale

Immutable bindings and immutable aggregate values keep the first-slice repair
loop local. A generated edit can reason about a name as a stable value in its
scope, and diagnostics do not need to explain hidden aliasing, destructive
updates, or order-dependent container state.

This choice also keeps contracts and hole constraints consistent with the
existing contract-expression and hole-satisfy decisions. Those predicates are
pure and may refer to immutable local bindings. If container updates were
destructive, then `old(...)`, postconditions, and hole candidate checks would
need a larger state and aliasing model before the first slice has proven the
basic tool loop.

The memory-management implication is intentionally modest. Immutable values may
share internal representation, but that sharing is not visible to user code.
Old and new versions of a container can coexist as ordinary values, and the
runtime only needs to preserve values that remain reachable. This lets the
first implementation choose the simplest viable collection strategy without
locking the language into that strategy.

## First-Slice Rules

- `let` introduces an immutable binding. Reassignment syntax is not available.
- First-slice records, lists, dictionaries, strings, `Option`, and `Result`
  values are immutable from user code.
- Container update operations return a new container value. They do not mutate
  an existing container in place.
- Implementations may use structural sharing or other representation
  optimizations when they do not change observable behavior.
- Equality for aggregate values is value-oriented where equality is supported.
  Pointer identity and object identity are not first-slice operations.
- The runtime provides automatic memory management for unreachable values.
- The language does not specify GC algorithm, collection timing, object layout,
  allocation strategy, or whether the first implementation uses a host runtime
  collector.
- Finalizers, destructors, weak references, mutable references, interior
  mutation, and explicit allocation or free operations are outside the first
  slice.
- The checker should report assignment-shaped or mutating-method-shaped syntax
  with targeted diagnostics when recovery can identify the intended construct.

## Open Details

The exact standard-library names for functional container operations remain
open. Examples include `list_append(items, item)`, `dict_insert(map, key,
value)`, or a future record-update form. The observable rule is that these
operations produce a new value.

Whether same-block shadowing is allowed remains a separate scoping decision.
For the first implementation, banning duplicate `let` names in the same block
would keep diagnostics simpler, but that rule is not required by this
mutability decision.

The first slice does not decide performance guarantees for persistent
containers. A later standard-library decision can specify expected complexity
or representation families once real examples show which operations matter.

## Consequence

Veln's first slice has a small runtime state model: values are immutable,
bindings do not change, container state is represented by new values, and
unreachable values are reclaimed by an implementation-chosen memory-management
strategy. This keeps GC choices flexible while preserving predictable behavior
for contracts, holes, diagnostics, and agent-generated repairs.
