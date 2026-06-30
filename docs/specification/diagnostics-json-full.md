# Diagnostics JSON Details

Status: routing

Use [diagnostics-json.md](diagnostics-json.md) first. Command-specific JSON
projection is documented in [json-output.md](json-output.md),
[run-json.md](run-json.md), and [test-json.md](test-json.md).

## Current Schema Diagnostic Boundary

Schema diagnostics cover parse rejection, primitive kind checks, field
references, validation predicates, dispatch payload eligibility, explicit
schema operation path resolution, and generated helper availability.
Schema-level mapping diagnostics are not current behavior because mapping
clauses are rejected by the parser.
