---
role: reference
authority: normative
update-when: The standard-library public API design rules for consumer scope, cardinality, polymorphism, outcomes, authority, progress, effects, export ownership, or design-review enforcement change.
---

# Standard-Library API Design Policy

This policy governs new and changed public standard-library APIs, including
source modules, re-exports, prelude names, compiler-known call targets, and
host-backed signatures. It also guides API reviews. It does not change the
current language behavior documented in `../specification/` or require every
existing API to migrate in an unrelated change.

The purpose is to preserve useful types and outcomes as callers compose
operations. Apply the rules to a representative caller, not just a declaration
in isolation. Keep a deliberate specialization when its semantics justify it.
The [ADT naming policy](standard-library-adt-naming.md) owns type and variant
names; the [test placement policy](toolchain-test-harness.md#test-placement-policy)
owns behavioral test locations.

## Public capability and canonical ownership

A public entry point must provide an identified reusable consumer capability.
A fixed sample schema, fixture constructor, benchmark hook, or compiler test
bridge is not a consumer capability merely because it is callable. Keep
internal examples in test inputs or separate test support. Library-provided
public testing APIs are appropriate when users need to test their own programs;
they follow the existing `_testing.veln` naming and distribution policy.

Give equivalent operations one canonical owner and implementation. Additional
names or facades need a discoverability, domain, convenience, or interoperability
purpose. When an alias is justified, share the implementation and specify
whether effects, results, ownership, and error behavior are identical. Do not
keep duplicate public modules solely because both spellings already exist.
Audit package exports and compiler-known names as well as `pub` declarations.

## Cardinality and operation families

For an operation that works on a runtime-sized collection, expose a collection
form instead of accumulating fixed-count overloads. Provide a direct single-item
form when it removes irrelevant collection construction or result indexing.
Equivalent single-item and singleton-collection calls must agree on payload,
errors, effects, cancellation, and committed side effects after the documented
result projection. Genuine differences require an explicit semantic reason.

This does not ban operations whose meaning is inherently binary, such as a
comparison or pairwise concatenation. Do not expand every function into a
complete matrix of optional features. Compare related operations and make
material differences in policy, controls, argument order, and result handling
explicit. Receiver count must not silently select a fairness or priority policy.

Use names for the consumer's operation. Distinguish waiting from producing a
deadline or duration value. Explain units, zero/negative inputs, empty inputs,
and immediate-attempt behavior where they affect the contract. Convenience
wrappers may supply a default, but must not silently change a related operation's
failure handling or privilege boundary.

## Preserve polymorphism where semantics permit it

A wrapper that only forwards a callback's value should preserve its result
and error types. Preserve the callback's effects through the supported effect
row mechanism. Do not impose `Result<(), String>` solely to make the wrapper
checkable when the operation has no unit-result or string-error requirement.
Specialized callbacks remain appropriate when the operation interprets that
specific result, such as an application protocol callback.

Check the implementation state of the needed language feature. A generic ADT,
a compiler-special generic helper, or an effect-row binder does not prove that
ordinary generic functions are supported through artifacts and execution.
When a general source signature is blocked, record that dependency in the
focused proposal. Do not advertise it as implemented, create many concrete
public substitutes without a consumer need, or attribute historical intent
without evidence. Remove a temporary limitation when its named dependency and
acceptance requirements are satisfied.

## Typed outcomes and authority

Use `Option`, `Result`, or an ADT to represent cases that callers must branch
on. Return the selected case's payload with the case instead of requiring a
string test followed by an independently fallible getter. Keep display strings,
protocol-defined strings, stable diagnostic identifiers, and explanatory
messages as text; they are not automatically bad interfaces.

Expected cancellation, timeout, closure, and partial failure must have a clear
caller-visible convention. If a wrapper intentionally turns an ordinary
outcome into an abrupt failure, give it a distinct documented consumer purpose;
do not introduce a parallel reporting family merely as an implementation
convenience. Distinguishing every condition is not mandatory: document any
intentional merged outcome and the recovery information it loses.

Encode public authority differences in types when operations require different
capabilities. An observation token must not type-check as permission to cancel
work if it cannot perform that operation. Keep resource validation for closed,
foreign, or malformed values; static roles do not replace dynamic state checks.
A bearer capability need not imply unique or linear ownership.

## Side effects, progress, and controls

Specify the side effects already committed on failure and the state left
usable for another operation. Preserve available progress when the operation
supports accounting or resumption. Distinguish an exact count from a known
lower bound when the backend cannot establish the total. Never invent zero
progress after a partial failure or present a lower bound as an exact count.
Transport commitment does not prove peer delivery or application-level safety.

A deliberately terminal convenience boundary may discard progress when it has
a stated consumer purpose and clearly cannot be used for resumption. Review
that choice against a progress-preserving primitive; do not force every unit
return into a universal progress abstraction.

For deadlines, cancellation, selection, and cleanup, define which already-set
conditions take precedence, where side effects commit, and which concurrent
orders are permitted. A losing attempt must not consume input or report a
successful transfer on another participant's behalf. Preserve FIFO, resource
ownership, and cleanup invariants across wrappers.

Effects describe the real boundary, including through convenience calls.
Do not hide effects to make a signature shorter or add value-dependent effect
claims unsupported by the checker. Specify whether a wrapper captures a
provider or uses a lexical one. Production defaults use real host effects;
controlled outcomes belong in separate scoped Veln support.

## Review evidence and enforcement

The specialized workflow is `$stdlib-api-design`, defined in
`.agents/skills/stdlib-api-design/SKILL.md` and required by the repository's
AGENTS rule. The author and reviewer must be able to trace
an affected public entry point to a consumer example, its outcome/authority
contract, and intended or implemented verification. A proposal distinguishes
planned evidence from current behavior and names real language dependencies.
Existing APIs are migrated through focused changes rather than silently
rewritten as part of enforcing this policy.

| Review observation | Required response |
| --- | --- |
| Sample-only source-callable export | Remove or keep internal unless a reusable public testing capability is established. |
| Arbitrary fixed-count API for a collection operation | Prefer single-item/collection forms or explain the semantic distinction. |
| Forwarding wrapper fixes unrelated payload/error types | Preserve types, or identify the semantic need or missing language dependency. |
| Branch string plus incompatible-case getter | Provide a typed projection with payload; retain genuine rendering APIs. |
| Observation value accepted as authority | Separate the capability types and add a negative type-check case. |
| Failure drops progress needed by its consumer | Preserve known/unknown progress or state why the boundary is deliberately terminal. |
| Duplicate public paths | Establish one implementation and a consumer purpose for each retained path. |

These are semantic review rules. No name-prefix ban, signature-arity limit,
required prose template, or heuristic string-return linter enforces them.
Such checks would reject valid binary operations, typed renderers, and useful
convenience wrappers while missing behavior hidden behind a good name.

Use existing bundle/export, checker, documentation/navigation, and stdlib
companion tests for mechanically checkable outcomes. Use independent expected
values and controlled failure schedules for state preservation. Backend and
host tests must protect a separate integration contract. Passing tests do not
establish that an API is useful; semantic review remains necessary.

Revisit a decision when its consumer, supported generic functions, effect
model, authority boundary, progress evidence, or canonical owner changes.
For an exception, explain the concrete consumer need and accepted loss at the
affected API or focused proposal; no new approval process is required. A
transitional exception names the capability that will retire it. Keep current
behavior under `../specification/` and incomplete changes under `../proposals/`.
