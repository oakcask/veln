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

`type.pipeline_target` reports a non-call expression on the right side of
`|>`. Its `details` include `phase`, `node_id`, `expected`, `actual`, and
`constraint: "pipeline_target"`.

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

For hole symbol queries with visible assignable bindings, an entry also
contains `candidates`. Each candidate contains:

- `candidate_id`
- `name`
- `type`
- `rank`
- `reason`
- `application_policy`

These records are ranked suggestions, not concrete edits. The
`application_policy` remains `manual_review_required`.
