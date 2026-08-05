---
role: specification
authority: normative
review-when: The diagnostic JSON contract or checked diagnostic examples change.
---

# Diagnostics JSON Details

Use [diagnostics-json.md](diagnostics-json.md) first. Command-specific JSON
projection is documented in [json-output.md](json-output.md),
[run-json.md](run-json.md), and [test-json.md](test-json.md).

## Current Schema Diagnostic Boundary

Schema diagnostics cover parse rejection, primitive kind checks, field
references, validation predicates, dispatch payload eligibility, explicit
schema operation path resolution, and generated helper availability.
Schema-level mapping diagnostics are not current behavior because mapping
clauses are rejected by the parser.

## Type Inference Diagnostics

`type.local_inference_incomplete` details identify the failed slot with
`slot_kind = "local_binding"` and `binding`, and report the current
`inferred_type` even when it still contains `unknown`.

`type.private_inference_incomplete` details identify the private function
boundary with `boundary = "private_function"`, identify the failed slot with
`slot_kind = "private_parameter"` and `parameter` or
`slot_kind = "private_return"`, report `missing_fact`, and report the current
`inferred_type` known at the failure point.

`type.inference_ambiguous` details identify the ambiguity slot with
`slot_kind`. Constructor type-context ambiguity uses
`slot_kind = "constructor_type"`, `constructor`, `inferred_type`, and
`constraint = "constructor_type_context"`. Empty collection ambiguity uses
`slot_kind = "empty_collection"`, `collection`, `inferred_type`, and
`constraint = "empty_collection_type_context"`. Match scrutinee domain
ambiguity uses `slot_kind = "match_scrutinee"`, `candidates`, and
`constraint = "match_constructor_pattern_domain"`.

Checked examples under `examples/specification/check/` pin these shapes for
local bindings, private helper parameters and returns, constructor ambiguity,
empty collection ambiguity, and match scrutinee ambiguity.

## Handler Diagnostics

Handler effect diagnostics use `phase = "effect"` and include `boundary`,
`handler`, `handled_effect`, nullable `operation`, and `reason`. Operation
clause diagnostics use `boundary = "handler_operation_clause"` and do not
emit a `provider` field. Unknown handled effects report `reason = "unknown_handled_effect"`
and add visible candidate effect declarations as related notes with `effect`
and `operations`.

The checked examples `handler-operation-signatures` and
`handler-operation-signatures-human` pin the structured and human related
context for missing, duplicate, unknown, mismatched, and recursive operation
clauses.

## Integer Bitwise Diagnostics

`type.invalid_shift_count` details contain `operator`, `actual_count`,
`minimum_count`, and `maximum_count`. The reported span is the literal count
expression.
