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

Malformed integer literals use `parse.integer_literal` with the complete
numeric candidate, parser context, accepted form, and non-cascading recovery;
related notes may identify the accepted digit set or prefix. Invalid literal
shift counts use `type.invalid_shift_count` with `operator`, `actual_count`,
`minimum_count`, and `maximum_count`; the span is the count expression.

Source identifier casing uses `name.invalid_case` with `phase`, `origin`,
`occurrence`, `name`, `name_class`, `required_initial`, and
`observed_initial`. Qualified written paths add zero-based `segment_index`.
Source-derived module paths use `origin: "source_path"`,
`occurrence: "path_segment"`, `source_path`, `source_kind`, `segment`, and
`segment_index`; `source_kind` is `regular`, `export`, `companion`,
`doctest`, or `generated`. Selected regular and companion sources may report
casing diagnostics alongside parse errors. A lowercase initial followed by an
invalid module-identifier character uses `module.invalid_source_path`.
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
