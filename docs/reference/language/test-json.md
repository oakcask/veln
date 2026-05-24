# Test JSON

This file specifies the implemented JSON output for `veln test --json`.

## Envelope

`veln test --json` emits schema version `veln-test-json/v0` with:

- `command`
- `status`: `passed`, `failed`, `blocked`, or `error`
- `selection`
- `summary`
- `diagnostics`
- `suite_errors`
- `cases`

## Selection

Selection fields are:

- `mode`: `discovered` or `explicit`
- `targets`
- `confidence`
- `reason`: `pattern_discovery` or `user_selected`

## Summary

Summary fields are:

- `total`
- `passed`
- `failed`
- `skipped`
- `todo`
- `blocked`
- `errors`

## Cases

Each case has:

- `id`
- `name`
- `kind`
- `status`
- `source`
- `reason`
- `failure`
- `events`
- `diagnostics`

Source `test` declarations use `case.kind: "test"` and a `source.node_id`
prefix of `test`. Ordinary functions use the `fn` prefix in other diagnostic
contexts but are not selected as test cases.

Captured stdio events use:

- `kind: "stdio"`
- `stream`: `stdout` or `stderr`
- `operation`: `print`, `println`, `eprint`, or `eprintln` when runtime tracing
  is available
- `text`
- `terminator`
- `sequence`
- `node_id`
- `span`

When the runtime trace is unavailable, output may be represented as aggregate
stdout or stderr events attached to the case source.
