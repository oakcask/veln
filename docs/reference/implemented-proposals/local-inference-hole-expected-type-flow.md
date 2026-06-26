# Local Inference Hole Expected-Type Flow

Status: implemented

This record keeps the completed typed-hole expected-type flow slice after the
behavior moved into the specification and executable examples. It is
historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types-full.md#inference](../../specification/types-full.md#inference).
- Current hole diagnostic rules:
  [../../specification/holes.md](../../specification/holes.md).
- Current JSON detail catalog:
  [../../specification/diagnostics-json.md](../../specification/diagnostics-json.md).
- Checked JSON example coverage:
  `../../../examples/specification/check/hole-expected-type-flow-json/case.toml`.
- Checked human diagnostic coverage:
  `../../../examples/specification/check/hole-expected-type-flow-human/case.toml`.

## Implemented Boundary

Typed holes receive concrete expected types from the same expression contexts
that already push expected types into implemented subexpression checking:
function return expressions, call arguments, record field initializers, `if`
branches, `match` arms, and ADT constructor payloads.

For those contexts, `hole.unfilled` names the rendered type in the human
primary message, records the same type in `details.expected_type`, records a
stable `details.expected_type_source`, and uses the expected type to build
advisory symbol candidate queries. Visible bindings of exactly matching or
assignable type stay ranked through the existing repair candidate path.

## Boundaries Preserved

- The slice does not infer public function signatures or exported boundaries.
- The slice does not add generalized generic function inference or
  user-defined higher-order helper inference.
- Candidate records remain advisory; no new repair application behavior is
  introduced.
- Holes without concrete expected type still report `unknown` and do not emit
  type-filtered symbol candidates.

## Completion Evidence

- Semantic tests cover expected-type flow into holes for return, call argument,
  record field, `if` branch, `match` arm, and constructor payload contexts.
- Executable specification examples cover JSON diagnostics with expected type,
  expected type source, and candidate query records.
- Executable specification examples cover the human primary message for a hole
  with a concrete expected type.
- Current specification pages document the implemented diagnostic behavior; the
  remaining proposal page keeps only incomplete local-inference work.

## Skip Unless Needed

- Do not read this page for current hole or inference rules.
- Use this record only when auditing why typed-hole expected-type flow is no
  longer listed as future proposal work.
