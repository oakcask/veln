# Self-Hosting Standard Library

Status: proposed

This page routes source-backed standard library proposal work whose behavior is
absent from the current specification. Implemented standard symbols, effects,
and compiler-known calls live in
[../specification/names-effects.md](../specification/names-effects.md).

## Current Target

Continue the source-backed pure prelude helper path. The implemented option
helper cluster is documented as current behavior in the specification:

- source-backed helper status and value semantics:
  [../specification/names-effects-full.md#prelude-helpers](../specification/names-effects-full.md#prelude-helpers)
- descriptor metadata boundary:
  [../specification/names-effects-full.md#compiler-known-descriptor-table](../specification/names-effects-full.md#compiler-known-descriptor-table)

The proposal area is the remaining descriptor-only pure helper set. A next
helper should keep its existing source-visible signature and semantics, add
embedded source metadata to the standard symbol descriptor, keep effects empty,
and preserve user-call-site diagnostics.

## Read First

- Current source surface:
  [../specification/source-surface.md](../specification/source-surface.md).
- Remaining proposal scope, candidate rules, and non-goals:
  [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates).

## Read When

- Moving another descriptor-only helper into the source-backed pure-helper
  model.
- Deciding which helpers must remain compiler-known until source-level effects
  or runtime boundaries are specified.
- Checking whether a standard-library idea is current behavior or still future
  proposal work.

## Boundary

- Stay within pure prelude helpers whose types and lowering already fit the
  existing descriptor-backed adapter path.
- Keep broad module loading, package management, subprocess pipelines,
  streaming I/O, public complexity guarantees, and source-level effect handlers
  out of this proposal.
- Keep current behavior documented under `../specification/`; keep proposal
  wording here until the implementation and specification both support it.

## Skip Unless Needed

- Do not use this page for current standard symbol behavior.
- Do not open the full proposal when the current target and boundary above
  answer the task.
