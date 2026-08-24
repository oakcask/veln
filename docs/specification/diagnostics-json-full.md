---
role: specification
authority: normative
update-when: The diagnostic JSON contract or checked diagnostic examples change.
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

## Name Diagnostics

Invalid source type, constructor, function declaration, test declaration,
public type alias declaration, public function alias declaration, and
value-binding names use `name.invalid_case`. Test declarations and public
function alias declarations use the function name class. Public type alias
declarations use the type name class. Public schema alias declaration names are
casing-neutral. The primary span is the complete written token. Details contain
`phase = "name"`,
`origin = "source"`, `occurrence`, `name`, `name_class`, `required_initial`,
and `observed_initial`. Executable JSON evidence includes the
`identifier-casing-json`, `identifier-casing-public-alias-declarations`,
`identifier-casing-invalid-alias-missing-target`,
`identifier-casing-invalid-alias-wrong-kind`,
`identifier-casing-invalid-alias-same-file-use`,
`identifier-casing-invalid-alias-duplicates`,
`identifier-casing-test-declaration`,
`identifier-casing-split-recovery-candidates`,
`identifier-casing-handler-callable-recovery`, and
`identifier-casing-pattern-binding-recovery` cases. The
`identifier-casing-inferred-callable-recovery` case checks
`details.typed_local_recovery_candidates` on ambiguous call-target recovery
diagnostics. The run cases
`identifier-casing-invalid-entry-json` and `identifier-casing-reachable-json`
check that pre-execution `run --json` casing failures use the same diagnostic
envelope shape. The `identifier-casing-mixed-json-diagnostics` run case checks
that the diagnostic envelope is also used when reachable pre-execution casing
and non-casing diagnostics are reported together.
The `identifier-casing-invalid-entry-wrong-arity-json`,
`identifier-casing-invalid-entry-unsupported-argument-json`, and
`identifier-casing-invalid-entry-conversion-json` run cases check that
selected entry casing diagnostics are emitted before entry argument validation
diagnostics.

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
