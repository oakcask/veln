# Discussion Result: Hole Diagnostic JSON Shape

## Picked Question

- What is the first JSON shape for hole diagnostics?

## Decision

Hole diagnostics should use the stable diagnostic envelope with
`kind: "hole"` and a prototype `details` payload focused on repair context.

The first slice should define one primary check-time diagnostic,
`hole.unfilled`, for a source-backed hole expression. `veln check --json`
should report `status: "partial"` when any unfilled hole remains, even if the
rest of the program is parseable and typeable. A hole diagnostic should be
`severity: "hint"` when it is useful repair guidance during `check`, and a
separate `error` diagnostic should be used when a selected operation is blocked
by the hole.

## First-Slice Shape

```json
{
  "id": "hole.unfilled",
  "severity": "hint",
  "kind": "hole",
  "message": "Hole requires a UserConfig value.",
  "span": {
    "file": "src/config.veln",
    "start": { "line": 12, "column": 14 },
    "end": { "line": 12, "column": 28 }
  },
  "details": {
    "label": "_config_parser",
    "expected_type": "UserConfig",
    "expected_type_source": "inferred",
    "constraints": [
      {
        "kind": "contract",
        "text": "result.port > 0"
      }
    ],
    "local_bindings": [
      {
        "name": "raw",
        "type": "String"
      }
    ],
    "candidate_queries": [
      {
        "kind": "symbol",
        "query": "fn(String) -> Result(UserConfig, _)"
      }
    ]
  },
  "related": [
    {
      "kind": "expected_type_origin",
      "message": "Return type declared here.",
      "span": {
        "file": "src/config.veln",
        "start": { "line": 8, "column": 28 },
        "end": { "line": 8, "column": 38 }
      }
    }
  ]
}
```

## Rationale

Agents need enough structure to choose the next edit without scraping prose:
the hole label, expected type, relevant constraints, visible bindings, and
candidate search hints. These fields describe the repair problem rather than a
specific editor workflow, so they remain useful for CLI output, editor
integrations, and tests.

Keeping this data under `details` follows the schema-stability decision. The
top-level envelope can stay stable while the language experiments with richer
constraint forms, candidate ranking, and type rendering.

## First-Slice Rules

- Anonymous `_` holes use `label: null`; named holes use the source label.
- `expected_type` is required and may be `"unknown"` only when binding or type
  analysis could not reach the hole.
- `expected_type_source` starts with `declared`, `inferred`, or `unknown`.
- `constraints`, `local_bindings`, and `candidate_queries` may be empty arrays,
  but should be present so agents do not have to infer omission semantics.
- `related` should point to the closest useful type origin, contract, or
  blocked operation when one is available.
- Runtime blocking remains a separate diagnostic such as
  `hole.runtime_blocked`; it may link back to one or more `hole.unfilled`
  diagnostics through `related`.

## Open Detail

The first implementation does not need stable candidate ranking. It only needs
candidate query records that are structured enough for tests to verify that the
checker understood the expected type and local context.

## Consequence

Typed-hole output becomes directly actionable for agents while preserving room
to evolve the deeper repair payload.
