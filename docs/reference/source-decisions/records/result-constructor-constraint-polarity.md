---
role: reference
authority: supporting
update-when: Function type variance, constructor generic constraint polarity, bounded join or meet selection, or explicit-context ambiguity behavior changes.
---

# Constructor Constraint Polarity

## Decision

Use contravariant function inputs and covariant function results when nested
constructor payloads contribute generic inference constraints. Record fields
preserve the incoming polarity. A resolved named-constructor argument composes
that polarity with the checked classification of its parameter. The current
[type specification](../../../specification/types.md#bounded-joins-and-meets)
then selects only a bounded join or meet that Veln can represent.

This choice gives function replacement its ordinary directional meaning: a
replacement result can be more specific, while a replacement input must still
accept every value accepted by the expected function. It also lets ordinary
constructor checking and inferred private signatures use one polarity model
without adding declaration-site variance syntax.

## Source basis and limits

The Rust Project's
[variance discussion in the Rustonomicon](https://doc.rust-lang.org/nomicon/subtyping.html#variance)
classifies function inputs as contravariant and function results as covariant.
Its function examples support the replacement interpretation used above.

Veln adopts that polarity, not Rust's type relation as a whole. Rust lifetime
subtyping and ownership do not determine joins or meets for Veln's nominal
ADTs and finite variant refinements. When Veln has no representable bounded
result, inference retains the existing explicit-context requirement instead of
importing another language's union, intersection, bottom, or lifetime rules.

The decision must be revisited if function assignment direction changes, if a
new type form adds another polarity boundary, or if Veln gains representable
join or meet results that remove the current explicit-context boundary.
