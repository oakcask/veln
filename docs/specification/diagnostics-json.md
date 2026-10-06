---
role: specification
authority: normative
update-when: The `veln check --json` diagnostic schema, human diagnostic alignment, stable diagnostic detail fields, or diagnostic producers change.
specification-coverage: usage=#usage; behavior=#diagnostic-families; limits=#limits
---

# Check JSON And Diagnostics

This page specifies the shared diagnostic envelope used by `veln check --json`
and by static failures from `run --json`. Test reports reuse the diagnostic
object shape within their own [test envelope](test-json.md).

## Usage

Run `veln check --json [INPUTS ...]` when a consumer needs structured analysis
diagnostics. The command emits one JSON object on stdout. Use
[commands.md](commands.md) for input discovery and exit behavior; use
[run-json.md](run-json.md) and [test-json.md](test-json.md) for completed
execution records.

## Envelope and common fields

The envelope has these fields:

| Field | Contract |
| --- | --- |
| `schema_version` | Number `1`. |
| `tool` | Object with string `name` and `version`. |
| `status` | `error` if any diagnostic has severity `error`; `partial` if there is no error and at least one `hole` diagnostic; otherwise `ok`. |
| `diagnostics` | Ordered array of diagnostic objects. |
| `summary` | Object with `diagnostic_count`, `by_severity`, and `by_kind`; each map counts the corresponding strings. |

Each diagnostic has non-null string fields `id`, `severity`, `kind`, and
`message`, plus `span`, `details`, and `related`. `span` is `null` for a
spanless diagnostic or an object with `file`, `start`, and `end`. `file` is
the source path. Each position has one-based `line` and Unicode-scalar
`column`, and zero-based byte `offset`; the end position is exclusive.
`details` is the producer's JSON
value and may be `null`. `related` is always an array of producer-supplied
JSON values.

The primary message names the failed fact at its reported span. Causes,
provenance, repair hints, and other locations belong in `related` or
structured `details`. Producers omit detail keys when the fact is unavailable;
consumers must preserve diagnostic and related-note order.

## Diagnostic families

`veln check --json` reports an over-limit cleanup form as
`parse.cleanup_nesting_limit`; the parser error makes the envelope status
`error`. A `defer` token used where an expression is required reports
`parse.expected_expression` at the token, with expression parser context and
skip-token recovery. Recovery preserves the boundary of a following
declaration. An unterminated `begin` or `defer` reports
`parse.begin_missing_end` or `parse.defer_missing_end`. The missing cleanup
delimiter does not consume an enclosing declaration's closing `end`, an
enclosing `if` expression's following `else` branch, or an enclosing `match`
expression's following arm. It does not merge a following top-level
declaration into the failed body.

A handler declaration with `handles` followed by another identifier that can
start its effect target reports `parse.expected_token` at the first `handles`.
Its message is `expected for`, its `details.expected` value is `["for"]`, and
its recovery strategy is `skip_token`; recovery treats the first `handles` as
the former separator and retains the following member path as the target. A
handler declaration that omits the separator reports the same diagnostic at
the effect target with `insert_token` recovery. That target can itself be the
ordinary identifier `handles` or a qualified path beginning with `handles::`.
Both recoveries retain the optional retained-effect list and the boundary of a
following declaration.

Malformed integer literals use `parse.integer_literal` with the complete
numeric candidate, parser context, accepted form, and non-cascading recovery;
related notes may identify the accepted digit set or prefix. Invalid literal
shift counts use `type.invalid_shift_count` with `operator`, `actual_count`,
`minimum_count`, and `maximum_count`; the span is the count expression.

Malformed variant-refinement-shaped type text uses
`parse.variant_refinement_type`. The
[malformed variant-refinement forms](source-surface.md#malformed-variant-refinement-forms)
define the rejected source shapes. The primary span selects the incomplete or
misplaced separator, segment, generic delimiter, or type argument. Parser
recovery retains the surrounding declaration and lossless source tree. This
diagnostic does not assert that a structurally valid base or final name resolves
semantically.

A value whose complete ADT variant set is not assignable at a direct local,
argument, branch, arm, or result boundary reports `type.variant_mismatch` at
that value expression. The primary message states the actual and expected
types. Details contain `phase`, `node_id`, `actual_type`, `expected_type`,
declaration-ordered `expected_variants`, `excluded_variants`, and `constraint`.
`excluded_variants` is an object with `form` and `variants`. The `listed` form
contains the declaration-ordered excluded finite set. The
`all_except_expected` form uses an empty `variants` array and states that every
base-ADT variant outside `expected_variants` is excluded, without copying the
complete ADT declaration into each diagnostic. A `variant_exclusion` related
note renders that fact for human output. One `expected_type_origin` related
note identifies the declaration or local annotation that supplied the
expectation. When a compiler-known helper infers a parameter expectation from
the call, the note instead identifies that helper at the call site. A nested
aggregate invariance failure with no truthful finite variant exclusion reports
ordinary `type.mismatch`. Its actual type preserves the inferred nested
refinement. This includes assigning an inferred record with a singleton-refined
field, or an inferred named aggregate with a refined type argument, to the
corresponding type that uses the base ADT. During aggregate inference, an
incompatible later contribution reports the aggregate position's accumulated
type as expected and preserves the later contribution's constructor refinement
as actual. The accumulated expected type contains only successfully typed
contributions. A contribution that already produced a diagnostic leaves that
type unchanged, does not produce a derivative aggregate mismatch, and cannot
change the expected type reported for a later contribution. A rejected call or
constructor expression follows this rule even when recovery can determine its
refined result type from the resolved declaration. The same rule applies when
one constructor payload uses a type parameter directly and another uses it
inside an invariant named type: the diagnostic reports the exact constraint
established by the other payload rather than widening the nested type. When one
payload contributes the same type parameter more than once, a conflict reports
the earlier occurrence's type as expected and the conflicting occurrence's
type as actual. None of that payload's occurrences update the accumulated type,
so a later payload is checked against only the constraints that were complete
before the rejected payload. If an earlier error leaves the value untyped,
`type.variant_mismatch` is omitted.

A valid constructor arm for a bare immutable match-scrutinee binding refines
that binding to the constructor singleton while checking the arm. Passing the
binding to a parameter that requires that singleton emits no
`type.variant_mismatch`. An invalid-cased, unresolved, inaccessible,
wrong-ADT, or payload-arity-mismatched constructor pattern does not establish
the refinement, so an incompatible use of the binding retains the ordinary
`type.variant_mismatch` behavior. The [type inference
rules](types.md#inference-rules) define the refinement scope and unsupported
scrutinee forms.

An unannotated `if` or `match` whose result refinements can join under the
[type inference rules](types.md#inference-rules) emits no diagnostic. After a
finite refinement join has started, a resolved branch or arm result that cannot
join retains the ordinary `type.mismatch` contract against the base ADT rather
than introducing a control-flow-specific diagnostic. Recovery abandons the
partial finite join and remains at the base ADT; later compatible refinements
do not resume it. When the first typed result cannot start a finite refinement
join, later results retain the existing first-result expectation, mismatch,
and recovery behavior.

Source identifier casing uses `name.invalid_case` with `phase`, `origin`,
`occurrence`, `name`, `name_class`, `required_initial`, and
`observed_initial`. Qualified written paths add zero-based `segment_index`.
Source-derived module paths use `origin: "source_path"`,
`occurrence: "path_segment"`, `source_path`, `source_kind`, `segment`, and
`segment_index`; `source_kind` is `regular`, `export`, `companion`,
`doctest`, or `generated`. Selected regular and companion sources may report
casing diagnostics alongside parse errors. A lowercase initial followed by an
invalid module-identifier character uses `module.invalid_source_path`.
When a selected documentation source cannot form a canonical virtual doctest
path, the same diagnostic reports the original documentation source. That
origin produces no generated doctest diagnostic path. Analysis continues for
the other selected sources, and `veln check --json` reports each rejected
origin once alongside their diagnostics. The record has kind `module`, the
original source span, and details containing
`phase: "module"`, `field: "module_identity"`, `source_path`, and the rejected
`segment`.
Recovery diagnostics do not create cascaded unresolved-name records.

Companion diagnostics expose `details.companion_path` and
`details.target_path` where applicable. Private companion effect and handler
wrong-target failures use `effect.private_companion_target` or
`handler.private_companion_target`; their details include companion and target
module fields plus `reason: "companion_target_mismatch"`. Effect failures
expose `companion_path`, `companion_target_module`, and `effect_module`; handler
failures expose `companion_path`, `companion_target_module`, and
`handler_module`. Public declarations use `module.companion_public_declaration`.
Exporting a test companion uses
`manifest.invalid_export` with `field: "lib.exports"`,
`reason: "test_companion"`, `source_path`, and `companion_path`.

Source-less compiler lookup failures are spanless
`toolchain.invalid_symbol_case` diagnostics with kind `toolchain` and details
`provider`, `name`, `name_class`, and `required_initial`. Invalid or duplicate
lookup keys and standard-symbol namespace mismatches use the same shape. They
remain toolchain failures rather than source `name.invalid_case`; human
messages identify invalid or duplicate lookup keys when applicable.

Schema diagnostics cover parse rejection, primitive kind checks, field
references, validation predicates, dispatch payload eligibility, explicit
schema operation path resolution, and generated helper availability. Mapping
clauses are rejected by the parser and therefore have no current schema
mapping diagnostic.

Type inference diagnostics include:

- `type.local_inference_incomplete`: `slot_kind: "local_binding"`,
  `binding`, and the current `inferred_type`, including `unknown` when present.
- `type.private_inference_incomplete`: `boundary: "private_function"`,
  `slot_kind: "private_parameter"` with `parameter`, or
  `slot_kind: "private_return"`, plus `missing_fact` and `inferred_type`.
- `type.inference_ambiguous`: `slot_kind` is `constructor_type`,
  `empty_collection`, or `match_scrutinee`. Constructor ambiguity adds
  `constructor`, `inferred_type`, and `constraint: "constructor_type_context"`;
  empty collection ambiguity adds `collection`, `inferred_type`, and
  `constraint: "empty_collection_type_context"`; match scrutinee ambiguity
  adds `candidates` and `constraint: "match_constructor_pattern_domain"`.

Deferred-block restriction diagnostics use kind `type`,
`phase: "type_check"`, `boundary: "deferred_block"`, and one of these stable
identifier and reason pairs:

| Identifier | Reason | Failed fact |
| --- | --- | --- |
| `defer.propagation` | `result_propagation` | Postfix `?` occurs inside a deferred block. |
| `defer.non_unit` | `non_unit_result` | The deferred block result is not `()`. |
| `defer.nested` | `nested_defer` | A deferred block registers another deferred block. |

Each record has one `related` entry with `kind: "repair_hint"`, a repair
message, and the containing deferred-block span. The primary span remains the
specific propagation `?` token, non-unit block result, or nested `defer` keyword that
failed.

Call-site execution-gate diagnostics use kind `type` and
`phase: "core_lowering"`:

| Identifier | Stable details | Failed fact |
| --- | --- | --- |
| `core.callsite_entry_unsupported` | `entry`, `boundary: "run_entry"` | The selected run entry declares the `callsite` modifier, but no Veln call expression can supply its hidden location. |
| `core.callsite_contract_call_unsupported` | `node_id`, `reason: "callsite_contract_call_unsupported"`, `callee` | A runtime contract on an ordinary function calls a call-site-aware function without enclosing hidden context to forward. |

The entry and contract-call records include a related
`runtime_support` note that states the unavailable runtime boundary. These
diagnostics stop `veln run` before backend launch. Direct and indirect calls in
ordinary function bodies, forwarding through call-site-aware wrappers, passing
call-site-aware callbacks to runtime-backed consumers, and direct built-in
references or call-site-aware direct calls in runtime contracts on
call-site-aware functions do not produce these diagnostics. The
ordinary-function contract diagnostic represents a deliberate boundary: its
enclosing function has no hidden call-site context to forward.

A schema declaration used as an ordinary local annotation type reports
`type.schema_reference` with `schema` and `use_kind: "local_annotation"`.
Exact-width and lowercase schema primitives in the same position report
`schema.exact_width_primitive` or `schema.lowercase_primitive` with `primitive`
and `reason: "local_annotation"`. These producer rules apply inside nested
`begin` and `defer` bodies and inside a `begin` used as a handler operation
clause.

Handler effect diagnostics use `phase: "effect"`, `boundary`, `handler`,
`handled_effect`, nullable `operation`, and `reason`. Operation-clause
diagnostics use `boundary: "handler_operation_clause"` and do not emit a
`provider`. Unknown handled effects use
`reason: "unknown_handled_effect"` and related notes containing candidate
`effect` and `operations` declarations.

Advisory hole candidate and application-policy fields are specified by
[repair-candidates.md](repair-candidates.md). Runtime result projections are
specified by [run-json.md](run-json.md).

## Limits

The envelope reports the diagnostics produced for the selected analysis set;
it does not include diagnostics from unselected sources or unloaded
dependencies. A static diagnostic envelope has no captured program stdout or
stderr fields. Human rendering may place related context in stderr while the
JSON fields remain unchanged. Diagnostic details are extensible by family, so
consumers must tolerate omitted and newly added detail keys.

The envelope implementation is `crates/veln-diagnostics/src/envelope.rs`;
family producers and focused CLI assertions provide the executable contract.
