# Discussion Result: Effect Declaration Boundary

Status: implemented

## Picked Question

- Should effects be declared on every impure function, inferred and displayed,
  or required only at public boundaries?

## Decision

Require explicit effect declarations at public boundaries in the first slice.
Private helpers may omit effect annotations; the checker should infer their
direct and transitive coarse effects and display them in diagnostics when those
effects matter to a public signature, contract, test selection, or hole repair.

Effectful built-ins, foreign calls, and runtime primitives must still have known
effect metadata. That metadata is part of the implementation surface, not a
requirement that every private source-level function repeat the same labels.

## Rationale

The first slice should make externally visible behavior reviewable without
forcing agents to maintain annotation noise inside local implementation code.
Public functions already require explicit parameter and return types; adding
explicit public effects keeps API contracts visible at the same boundary and
gives downstream diagnostics a stable place to report capability drift.

Requiring annotations on every impure helper would make small edits more
expensive. An agent that adds one `fs` read deep in a helper chain would need to
touch multiple private signatures before the program communicates the important
fact: the public operation now reads the file system. Inference can carry that
fact upward, and the checker can ask for an explicit public declaration only
where the behavior becomes part of a caller-facing contract.

Pure inference with display-only effects is also too weak for the stated design
anchors. If public APIs can silently gain `net`, `db`, `time`, or `random`, the
repair loop shifts risk to reviewers and CI. A public-boundary rule catches
that drift while preserving compact local code.

## First-Slice Rule

- Public functions must declare their coarse effect set, even when the set is
  empty.
- Private functions may omit effect annotations. The checker infers direct and
  transitive effects for them.
- A public function body whose inferred effect set is not covered by its
  declared effect set produces a `kind: "effect"` diagnostic.
- The diagnostic should include the missing effect labels, the public signature
  span, and related spans for the closest operations or calls that introduced
  the effects.
- Effectful built-ins, foreign functions, and runtime primitives must carry
  known effect metadata so inference has trustworthy leaves.
- First-slice labels stay coarse, such as `stdio`, `fs`, `net`, `db`, `time`,
  `random`, and `process`.
- The checker may warn about explicit private annotations that disagree with
  inference, but private annotations are optional and are not required for the
  first implementation.

## Open Detail

The first-slice grammar resolves the source spelling as `effects [...]` after
the return type. This decision still owns the rule that public functions must
declare the coarse set while private helpers may rely on inference.

Access modes such as `db: read` and `db: write` remain unresolved. The first
slice should not require them before coarse labels have proven useful.

The display policy for large transitive effect chains also remains unresolved.
This decision only requires enough related-span information to identify the
nearest useful cause of an undeclared public effect.

## Consequence

Public APIs gain explicit side-effect contracts early, while private code stays
cheap to generate and repair. Agents can rely on `veln check` to surface effect
drift at module boundaries without needing a complete effect-system design in
the first implementation.
