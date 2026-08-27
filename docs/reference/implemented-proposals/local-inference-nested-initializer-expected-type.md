---
role: implementation-record
authority: supporting
update-when: The completed proposal record, evidence links, or current specification authority changes.
---

# Local Inference Nested Initializer Expected Type


This record keeps the completed nested initializer expected-type propagation
slice after the behavior moved into the specification and executable examples.
It is historical evidence, not the source for current behavior.

## Read First

- Current type inference summary:
  [../../specification/types.md](../../specification/types.md).
- Current full inference rules:
  [../../specification/types.md#inference](../../specification/types.md#inference).
- Checked example coverage:
  `../../../examples/specification/check/local-let-inference/`.

## Implemented Boundary

Concrete expected record field types propagate into nested record literal field
initializers.

Concrete expected source-declared constructor payload types propagate into
nested payload initializer expressions.

When each enclosing record field or constructor payload type is concrete,
empty `Vec<T>` literals, `Nil` for `List<T>`, empty dictionary literals, and
source-declared nullary constructors inside nested initializer expressions use
that concrete context.

## Boundaries Preserved

- The slice does not infer public function signatures.
- The slice does not add generalized let-polymorphism, traits, implicit
  conversions, or cross-module inference.
- Expected types that still contain `unknown` are not concrete enough to
  choose empty collection or nullary constructor types.
- Ambiguous record fields or constructor descriptors keep the existing
  incomplete, ambiguous, or mismatch diagnostics.

## Completion Evidence

- Executable specification examples cover record literals nested inside record
  literals.
- Executable specification examples cover source-declared constructor
  payloads nested inside record literals.
- Executable specification examples cover record literals nested inside
  source-declared constructor payloads.
- Executable specification examples cover source-declared constructor
  payloads nested inside constructor payloads.
- Current specification pages document the inference rule; the remaining
  proposal page keeps only incomplete local-inference work.

## Skip Unless Needed

- Do not read this page for current inference rules.
- Use this record only when auditing why nested initializer expected-type
  propagation is no longer listed as planned proposal work.
