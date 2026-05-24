# Check JSON

Status: implemented
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
- `kind`: `parse`, `name`, `type`, `contract`, `effect`, `lint`, `hole`, or
  `doc`
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

Name diagnostic `details` are stable for unresolved names:

- `phase`
- `node_id`
- `symbol`
- `namespace`
- `resolution_status`
- `candidates`

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

Effect diagnostic `details` are stable for `effect.missing_public`:

- `phase`
- `node_id`
- `effect`
- `boundary`
- `declared_effects`
- `inferred_effects`
- `provenance`
- `provenance_truncated`

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
