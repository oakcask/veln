# Discussion Result: Contract Blame Boundary

Status: implemented

## Picked Question

- Should failed `require` usually blame the caller and failed `ensure` usually
  blame the implementation in the first version?

## Decision

Use `require` as a caller-side obligation and `ensure` as an implementation-side
obligation in the first version.

When a checked `require` clause fails at a call boundary, diagnostics should
default to blaming the caller for providing arguments or state outside the
function's accepted domain. When a checked `ensure` clause fails after the
function body produces a value, diagnostics should default to blaming the
implementation for not satisfying its advertised result contract.

This is a default blame rule, not a proof of fault. Diagnostics should use
language such as `blame: "caller"` and `blame: "implementation"` as routing
metadata, while keeping the human message focused on the failed clause and
related spans.

## Rationale

The first slice needs contract failures to be actionable without requiring a
full verification system. A caller/implementation split gives agents a direct
repair route: inspect the call site when a precondition fails, and inspect the
function body when a postcondition fails.

This convention also matches the public-boundary type rule. Public signatures
describe what callers may rely on; `require` narrows acceptable inputs, and
`ensure` promises properties of successful outputs. If both forms produce the
same generic contract failure, an agent has to infer the likely edit location
from prose and stack shape.

The rule should stay conservative. A failed `require` can still be caused by a
buggy wrapper that forwarded bad data, and a failed `ensure` can still expose an
overly strong or stale contract. The diagnostic should point to both the failed
clause and the nearest useful call or implementation span when available.

## First-Slice Rule

- `require` failures default to `kind: "contract"` diagnostics with
  `details.blame: "caller"`.
- `ensure` failures default to `kind: "contract"` diagnostics with
  `details.blame: "implementation"`.
- `invariant` runtime failures use caller blame at function entry and
  implementation blame at function return. Static invariant diagnostics use
  `details.blame: "caller_or_implementation"`.
- Diagnostics should include the failed clause text or structured expression,
  the clause span, and a related span for the likely repair location when one is
  available.
- Static contract diagnostics should use the same blame values when analysis can
  prove or strongly suspect a violation without running code.
- Blame metadata is routing guidance for repair tools and reviewers. It must not
  suppress related spans or make the diagnostic message claim certainty beyond
  the available evidence.

## Open Detail

The exact contract expression grammar remains unresolved. This decision only
requires that each checked contract clause carries enough source identity for
diagnostics to report which clause failed and where a repair should start.

The first implementation also needs to decide how contract failures are
represented during `run` and `test`: as process failures, structured runtime
errors, or test assertions. Whatever runtime shape is chosen should preserve the
same blame metadata used by `check --json`.

## Consequence

Contract diagnostics can guide agents toward the right side of an API boundary
before the language has a richer verifier. This keeps the repair loop short
while leaving room to refine contracts, invariants, and runtime error handling
later.
