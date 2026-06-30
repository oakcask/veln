# Execution Details

Status: routing

Use [execution.md](execution.md) first. Checked cases under
`../../examples/specification/` and focused crate tests are the primary
execution evidence.

## Current Schema Helper Boundary

Generated binary schema decode, validate, decode-step, encode, and encode-step
helpers use schema-local visible record shapes. Representation-only fields are
validated by the helper and omitted from the visible record. Ordinary Veln
functions perform projection between schema-local records and domain records.

Schema-level mapping clauses are rejected before execution and do not affect
generated helper runtime behavior.

## Read When

- Use this page only as a stable route for old links.
- Prefer the checked examples named from [execution.md](execution.md) for
  current observable behavior.
