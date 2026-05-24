# Discussion Result: Check JSON Details Fields

Date: 2026-05-24

## Picked Question

- What exact first-slice `veln check --json` kind-specific `details` fields are
  required for parse, type, contract, effect, and hole diagnostics?

## Decision

Use one stable top-level envelope plus small, always-present prototype
`details` payloads for each first-slice diagnostic family.

The `details` payload is not yet compatibility-stable, but golden tests should
assert its first implementation shape. Every payload should contain only facts
that the checker already computed: phase, source node identity when available,
expected and actual facts, recovery or provenance evidence, and repair-routing
metadata. Do not embed free-form explanation paragraphs in `details`; keep prose
in `message` and put structured evidence in `related` or typed arrays inside
`details`.

## Shared Detail Rules

- Use snake_case JSON keys.
- Include `phase` in every `details` object with one of `parse`, `type`,
  `contract`, `effect`, or `hole`.
- Include `node_id` when the diagnostic is attached to a recovered surface AST
  node; use `null` for unrecovered parse errors.
- Include empty arrays rather than omitting list fields.
- Keep spans in the stable `span` and `related` envelope fields unless a nested
  value itself needs a secondary span.
- Use source-relative file names in spans; never serialize machine-local
  absolute paths.
- Prefer stable symbolic values over rendered text for routing, while retaining
  rendered type or predicate strings where they are the only first-slice
  representation.

## Parse Details

Parse diagnostics should expose what was found, what the parser was expecting,
and how recovery proceeded.

```json
{
  "phase": "parse",
  "node_id": null,
  "parser_context": "function_body",
  "unexpected": {
    "kind": "keyword",
    "text": "else"
  },
  "expected": ["end", "let", "match", "expression"],
  "recovery": {
    "strategy": "skip_token",
    "anchor": "end",
    "dropped_token_count": 1
  }
}
```

Required fields: `phase`, `node_id`, `parser_context`, `unexpected`,
`expected`, and `recovery`.

`recovery.strategy` starts with `none`, `skip_token`, `insert_token`,
`close_block`, or `synchronize_to_anchor`.

## Type Details

Type diagnostics should expose the expected type, actual type, and the source of
each fact.

```json
{
  "phase": "type",
  "node_id": "expr-42",
  "expected_type": "Int",
  "actual_type": "String",
  "expected_type_source": "declared_return",
  "actual_type_source": "inferred_expression",
  "constraint": "assignable",
  "origin_node_ids": ["fn-7", "expr-42"]
}
```

Required fields: `phase`, `node_id`, `expected_type`, `actual_type`,
`expected_type_source`, `actual_type_source`, `constraint`, and
`origin_node_ids`.

`expected_type` and `actual_type` may be `"unknown"` only when earlier errors
prevented analysis. `constraint` starts with `assignable`, `call_argument`,
`return_value`, `operator_operand`, `match_arm`, or `contract_predicate`.

## Contract Details

Contract diagnostics should expose which clause failed validation or static
discharge, whether runtime checking remains required, and the default blame
side.

```json
{
  "phase": "contract",
  "node_id": "contract-9",
  "clause": "ensure",
  "predicate_text": "out.port > 0",
  "validation_status": "valid_unknown",
  "obligation_status": "runtime_required",
  "reason": "not_statically_discharged",
  "blame": "implementation",
  "runtime_required": true,
  "referenced_bindings": [
    {
      "name": "out",
      "kind": "result"
    }
  ]
}
```

Required fields: `phase`, `node_id`, `clause`, `predicate_text`,
`validation_status`, `obligation_status`, `reason`, `blame`,
`runtime_required`, and `referenced_bindings`.

`validation_status` starts with `valid`, `valid_unknown`, or `invalid`.
`obligation_status` starts with `discharged`, `runtime_required`, or
`failed_static`. Invalid predicates use `runtime_required: false`.

## Effect Details

Effect diagnostics should expose the missing or inconsistent effect label, the
public boundary being checked, and a bounded provenance slice.

```json
{
  "phase": "effect",
  "node_id": "fn-3",
  "effect": "stdio",
  "boundary": "public_function",
  "declared_effects": [],
  "inferred_effects": ["stdio"],
  "provenance": [
    {
      "node_id": "call-11",
      "kind": "direct_call",
      "symbol": "stdio.print"
    }
  ],
  "provenance_truncated": false
}
```

Required fields: `phase`, `node_id`, `effect`, `boundary`,
`declared_effects`, `inferred_effects`, `provenance`, and
`provenance_truncated`.

`boundary` starts with `public_function`, `entry_point`, or `test`. The first
slice should report missing public effects with `id: "effect.missing_public"`.

## Hole Details

Hole diagnostics keep the shape from
[Hole Diagnostic JSON Shape](result-hole-diagnostic-json-shape.md), with the
shared `phase` and `node_id` fields added.

```json
{
  "phase": "hole",
  "node_id": "hole-5",
  "label": "_config_parser",
  "expected_type": "UserConfig",
  "expected_type_source": "inferred",
  "constraints": [
    {
      "kind": "contract",
      "text": "candidate.port > 0"
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
}
```

Required fields: `phase`, `node_id`, `label`, `expected_type`,
`expected_type_source`, `constraints`, `local_bindings`, and
`candidate_queries`.

## Rationale

LSP and SARIF both support a small common diagnostic record with code, severity,
message, location, and related information while leaving room for structured
tool-specific data. Veln should follow that split: the stable envelope is the
integration contract, while `details` carries first-slice checker evidence.

Compiler-error research supports making diagnostics explanatory and
resolution-oriented, but that does not mean long prose belongs in the JSON
payload. Barik, Ford, Murphy-Hill, and Parnin show that useful explanations
need evidence and, when possible, a resolution. For Veln, the agent-consumable
version of that argument structure is expected versus actual facts, provenance,
and bounded candidate or repair context.

Parser recovery work supports reporting both expected input and the recovery
action, because parse diagnostics should remain useful after the checker
continues over partial source. Typed-hole and type-directed completion work
supports exposing expected type, local bindings, constraints, and candidate
queries for incomplete expressions. Program slicing and question-centered
debugging support bounded provenance rather than full internal graphs for
effect and contract explanations.

## First-Slice Rules

- Golden diagnostics should assert required keys and symbolic enum values, not
  exact prose in `message`.
- A diagnostic may set `status: "partial"` when earlier errors limit analysis,
  but it should still fill every required `details` key with `"unknown"`,
  `null`, or an empty array as appropriate.
- `related` remains the preferred place for source spans such as declaration
  origins, conflicting definitions, contract clauses, or effect call sites.
- `details` values should be deterministic across machines and runs for the
  same source and tool version.
- Kind-specific `details` changes are prototype-compatible until the
  corresponding diagnostic kind is promoted out of prototype status.

## Open Details

The first slice does not define code actions, automatic repairs, SARIF export,
or LSP transport. The payload is deliberately close enough to map into those
formats later without committing the CLI JSON output to either protocol.

## Consequence

`veln check --json` now has enough concrete kind-specific structure for parser,
checker, contract, effect, and hole golden tests, while preserving the
previously decided compatibility boundary around the top-level diagnostic
envelope.

## References

- Microsoft. (2026). *Language Server Protocol Specification - 3.17:
  Diagnostic*.
  https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#diagnostic
- Fanning, M. C., & Golding, L. J. (Eds.). (2020). *Static Analysis Results
  Interchange Format (SARIF) Version 2.1.0*. OASIS Standard.
  https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/sarif-v2.1.0-os.html
- Barik, T., Ford, D., Murphy-Hill, E., & Parnin, C. (2018). How Should
  Compilers Explain Problems to Developers? *ESEC/FSE 2018*.
  https://doi.org/10.1145/3236024.3236040
- Medeiros, S. Q. de, Alvez Junior, G. de A., & Mascarenhas, F. (2019).
  *Automatic Syntax Error Reporting and Recovery in Parsing Expression
  Grammars*. arXiv:1905.02145. https://arxiv.org/abs/1905.02145
- Omar, C., Voysey, I., Chugh, R., & Hammer, M. A. (2019). Live functional
  programming with typed holes. *Proceedings of the ACM on Programming
  Languages*, 3(POPL), 1-32. https://doi.org/10.1145/3290327
- Weiser, M. (1982). Programmers use slices when debugging. *Communications of
  the ACM*, 25(7), 446-452. https://doi.org/10.1145/358557.358577
