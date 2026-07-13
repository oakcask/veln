# Recursive Format-Neutral Schema Encode Shapes

Status: implemented

## Result

Generated `byte_encode_<schema>` helpers and explicit
`encode Schema from value` expressions use one recursive eligibility rule for
format-neutral schema fields. Eligible shapes contain `Int`, `Bool`, `Float`,
or `String` leaves, anonymous records, `Option<T>`, `List<T>`, `Vec<T>`,
`Dict<String, T>`, `Result<Ok, Err>`, and visible same-module or public
imported source ADTs. Every child and ADT constructor payload must satisfy the
same rule.

The traversal terminates when a recursive source ADT refers back to an ADT
already being checked, while still validating the revisited ADT's instantiated
type arguments. Container nesting has no separate depth limit.
Non-`String` dictionary keys, functions, unresolved or private imported ADTs,
and other unsupported source types remain outside the boundary.

## Evidence

- `../../../examples/specification/run/format-neutral-schema-recursive-encode-shapes/`
  executes both generated-helper and explicit-expression paths through mixed
  deep containers and a recursive source ADT.
- Focused semantic tests cover generated helper resolution, explicit encode
  resolution, checked-core lowering, typed-IR lowering, and recursive ADT
  termination, including an accepted recursion whose type arguments keep
  growing through eligible options and rejection when a recursive reference
  changes its type arguments to an unsupported function shape.
- Existing negative specification cases continue to check non-`String`
  dictionary keys, function payloads, and unavailable source ADTs with
  `schema.format_neutral_encode_helper` at the schema field span.

Current behavior is specified in `../../specification/source-surface.md` and
`../../specification/execution.md`.
