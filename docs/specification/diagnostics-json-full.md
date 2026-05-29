# Check JSON

This file specifies the implemented JSON output for `veln check --json`.

## Envelope

`veln check --json` emits this top-level object:

```json
{
  "schema_version": 1,
  "tool": {"name": "veln", "version": "..."},
  "status": "ok | partial | error",
  "diagnostics": [],
  "summary": {
    "diagnostic_count": 0,
    "by_severity": {},
    "by_kind": {}
  }
}
```

Status is `error` if any diagnostic has severity `error`, `partial` if there
are no errors and at least one hole diagnostic, and `ok` otherwise.

## Diagnostics

Every diagnostic has:

- `id`
- `severity`: `error`, `warning`, `info`, or `hint`
- `kind`: `parse`, `module`, `name`, `type`, `contract`, `effect`, `lint`,
  `hole`, or `doc`
- `message`
- `span`, or `null`
- `details`
- `related`

Spans use one-based `line` and `column` plus byte `offset`.

## Stable Details

Parse diagnostic `details` are stable for the implemented slice:

- `phase`
- `node_id`
- `parser_context`
- `unexpected`
- `expected`
- `recovery`

Contract predicate syntax diagnostics use the same parse-phase detail shape.
`require` and `ensure` predicate parse failures use `kind: "contract"` with
`id: "parse.contract_predicate"` so contract-specific failures remain grouped
with other contract diagnostics.
Malformed call argument lists report `parse.call_argument` when an argument is
followed by another token without the required `,` or `)`. The recovery detail
uses `strategy: "insert_token"` with `anchor: ","` when the parser continues by
treating the next token as another argument.
Body expression lines report `parse.expected_newline` when a complete
expression is followed by another token before the line ends. The recovery
detail uses `strategy: "insert_token"` with `anchor: "newline"`.
Malformed `let` patterns report `parse.pattern` when the pattern parser leaves
extra tokens before the `=`. The recovery detail uses
`strategy: "insert_token"` and expects `pattern end`.

Name diagnostic `details` are stable for unresolved and duplicate names:

- `phase`
- `node_id`
- `symbol` for unresolved references
- `name` for duplicate declarations
- `namespace`
- `resolution_status` and `candidates` for unresolved references
- `first_node_id` for duplicate declarations

`name.duplicate` reports the duplicate declaration span as the primary span.
The first declaration appears in `related` with `kind: "duplicate_origin"`.

Module diagnostic `details` are stable for missing source module identity and
module metadata drift:

- `phase`
- `node_id`
- `field`
- `expected_owner`
- `observed_owner`
- `canonical_owner`
- `derived_owner`
- `expected_value`
- `observed_value`
- `manifest_path`
- `source_path`

`module.missing_identity` reports the first `use` declaration as the primary
span and puts the repair hint in `related`.

`module.metadata_drift` reports the manifest module name as the primary span.
The source `mod` declaration appears in `related` with
`kind: "canonical_owner"` when it exists.

Type diagnostic `details` are stable for public-signature, invalid-annotation,
and type-mismatch diagnostics:

- `phase`
- `node_id`
- `expected_type`
- `actual_type`
- `expected_type_source`
- `actual_type_source`
- `constraint`
- `origin_node_ids`

When a body omits its final expression line, return checking uses `()` with
`actual_type_source: "implicit_unit"`.

`type.private_inference_incomplete` reports a private parameter or private
return type whose omitted annotation did not infer to a concrete type. Its
`details` include `phase`, `node_id`, `boundary: "private_function"`,
`missing_fact`, and `inferred_type`. Repair hints are related notes.

`type.pipeline_target` reports a target on the right side of `|>` that is not
a call, or is a call whose callee is not a name path. Its `details` include
`phase`, `node_id`, `expected`, `actual`, and
`constraint: "pipeline_target"`.

`type.method_call` reports method-call-shaped syntax such as
`value.method(args)`. Its primary span is the method name. Its `details`
include `phase`, `node_id`, `expected: "function_call"`,
`actual: "method_call"`, `constraint: "call_target"`, and `method`. A related
note carries the canonical named-call repair direction.

`type.match_non_exhaustive` reports a `match` expression over a finite built-in
domain that does not cover every case and has no catch-all arm. Its primary
span is the `match` expression, and the primary message names the first missing
case. Its `details` include `phase`, `node_id`, `scrutinee_type`,
`missing_case`, and `constraint: "match_exhaustiveness"`. Related notes include
`kind: "scrutinee_type"` for the scrutinee span and `kind: "covered_case"` for
arms that prove partial coverage.

Checked-core executable blockers that `check` can prove before runtime are
reported as error diagnostics with `kind: "type"`. The implemented blockers
are `core.missing_expression`, `core.call_arity_mismatch`,
`core.result_constructor_arity_mismatch`, and
`core.option_constructor_arity_mismatch`. For missing expressions, the primary
span is the missing expression placeholder. For arity mismatches, the primary
span is the blocked call or constructor expression. Their `details` include:

- `phase: "core_lowering"`
- `node_id`
- `reason`

`core.missing_expression` details also include `expected_type` when the missing
expression had one.

Arity mismatch details also include:

- `facts.expected_argument_count`
- `facts.actual_argument_count`

Concurrency runtime blockers also include `facts.symbol`.

Effect diagnostic `details` are stable for `effect.missing_public`:

- `phase`
- `node_id`
- `effect`
- `boundary`
- `declared_effects`
- `inferred_effects`
- `provenance`
- `provenance_truncated`
- `provenance_paths`

Each `provenance_paths` entry contains:

- `effect`
- `entries`
- `truncated`
- `hidden_frame_count`
- `omitted_path_count`

Each path entry contains:

- `kind`
- `node_id`
- `symbol`
- `span`

Effect diagnostic `details` are stable for `effect.unknown`:

- `phase`
- `node_id`
- `effect`
- `boundary`
- `declared_effects`
- `known_effects`

Contract diagnostic `details` are stable for implemented contract validation:

- `phase`
- `node_id`
- `clause`
- `predicate_text`
- `validation_status`
- `obligation_status`
- `reason`
- `blame`
- `runtime_required`
- `referenced_bindings`

Static diagnostic `blame` is `caller` for `require`, `implementation` for
`ensure`, and `caller_or_implementation` for `invariant`; runtime invariant
failures report the concrete entry or return blame.

Hole diagnostic `details` are stable for unfilled holes:

- `phase`
- `node_id`
- `label`
- `expected_type`
- `expected_type_source`
- `constraints`
- `local_bindings`
- `candidate_queries`

Each `candidate_queries` entry is advisory and contains:

- `kind`
- `candidate_status`
- `application_policy`
- `query`

For satisfy-constrained holes, a candidate query also contains:

- `satisfy_predicate`
- `satisfy_candidate_binding`

For hole symbol queries with visible assignable bindings, an entry also
contains `candidates`. Each candidate contains:

- `candidate_id`
- `name`
- `type`
- `rank`
- `reason`
- `application_policy`
- `edits`
- `target`
- `edit_summary`
- `evidence`
- `known_limits`
- `blocking_obligations`
- `verification_hint`
- `application_status`

Each edit contains `kind: "replace"`, `span`, and `replacement`. The edits are
concrete but unapplied, and `application_status` is `unapplied`. `target`
contains the hole node ID and span. `evidence` records the passed type fact,
ranking fact, unrun verification fact, and satisfy fact when a `satisfy`
predicate applies.
`known_limits` and `blocking_obligations` keep advisory candidates from being
mistaken for applied or fully verified edits. The default `application_policy`
remains `manual_review_required`; the direct reflexive, tautological, and
`require`-matched satisfy subsets may use `safe_repair_candidate`. Safe repair
candidates still carry a verification blocking obligation until the suggested
verification command has been run after applying the candidate edit. Candidates
for satisfy-constrained holes also contain `satisfy_status`, either
`statically_satisfied` or `blocked_until_discharged`. Implementations may bound
ordinary manual-review candidates, but statically satisfied candidates are
retained even when they sort after that ordinary bound.

Satisfy entries in `constraints` contain `repair_status`, either
`statically_satisfied` when a type-compatible visible symbol candidate already
discharges the predicate, or `blocked_until_discharged` otherwise.

Semantic satisfy diagnostics use hole diagnostic detail objects with
`phase: "hole"`, `node_id`, `candidate_binding`, and `predicate_text`. Type mismatch
details also include `expected_type` and `actual_type`; unsupported construct
details include `reason`; missing field details include `base_type` and
`field`. Unresolved names in satisfy predicates use the normal name diagnostic
shape with `namespace: "satisfy_predicate"`.

Doc diagnostic `details` are stable for doctest metadata diagnostics:

- `kind: "doctest_metadata"`
- `attribute` when the diagnostic names one malformed attribute
- `fence` for unknown attributes
- `stream` for invalid output stream values

`doctest.unknown_metadata` reports an unsupported `veln` or `veln-output`
fence attribute at the fence line. `doctest.invalid_metadata` reports an empty
`error=`, empty runtime expectation metadata, unsupported runtime expectation
kind, missing runtime contract `clause` or `predicate`, missing `stream`, or
output stream value other than `stdout` or `stderr`.
`doctest.expected_failure_missing` reports a `veln fail` fence whose generated
negative example produced no error diagnostic.
