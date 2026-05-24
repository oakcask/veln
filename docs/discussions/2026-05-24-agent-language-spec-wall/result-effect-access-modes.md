# Discussion Result: Effect Access Modes

Date: 2026-05-24

## Picked Question

- Should effect labels include access mode such as `db: read` and `db: write`
  in the first slice?

## Decision

Do not require access-mode-qualified effect labels in first-slice source
declarations. Public effect declarations should use coarse labels such as
`stdio`, `db`, `fs`, `net`, `time`, `random`, and `process`.

The checker may attach experimental access-mode metadata to diagnostics when it
can do so cheaply and confidently. For example, a related span for a database
operation may report `effect: "db"` and `access: "read"` in its prototype
`details` payload, but the public declaration is still satisfied by `db`.

## Rationale

The first effect-system job is to catch surprising capability drift at public
boundaries. Coarse labels are enough to answer whether a public API started
touching standard I/O, the database, file system, network, clock, randomness,
or process state. Requiring `read` and `write` modes immediately would make the
language surface larger before the implementation has enough examples to prove
that the extra precision improves repair loops.

Access modes are also harder to classify consistently than the coarse labels.
A database query is often read-only, but a stored procedure, cache population,
lock acquisition, migration check, trigger, or audit hook may make the operation
observably writeful. A file-system operation can read metadata while also
affecting access time or a watch state. If first-slice declarations promise
more precision than the checker can defend, agents will start repairing code
toward misleading annotations instead of useful behavior boundaries.

Keeping access mode out of the required declaration syntax preserves a small
public contract while still collecting evidence. Diagnostics can surface known
operation modes as advisory repair context, and later design work can promote
the mode dimension only if examples show that coarse labels cause real review
or test-selection friction.

## First-Slice Rule

- Source-level public effect declarations use coarse labels only.
- `db: read`, `db: write`, `fs: read`, and similar qualified labels are not
  accepted as required first-slice declaration syntax.
- Effect metadata for built-ins, foreign calls, and runtime primitives may
  record an optional access mode when the mode is trustworthy.
- `veln check --json` may include access-mode metadata inside effect diagnostic
  `details`, but the stable diagnostic envelope remains keyed by the coarse
  effect label.
- Missing-effect diagnostics should report the coarse label needed at the
  public boundary and may show access modes only as related operation context.
- The syntax space for qualified effects should remain reserved so a later
  design can add capability qualifiers without conflicting with first-slice
  code.

## Open Detail

The exact shape of advisory access-mode metadata in JSON diagnostics is not
fixed here. Until the kind-specific `details` payload stabilizes, access modes
should be treated as best-effort context rather than a compatibility guarantee.

This decision also does not define a permission lattice. It deliberately avoids
answering whether `write` implies `read`, whether `append` is separate from
`write`, or whether effect modes should eventually be user-extensible.

## Consequence

The first implementation can enforce public effect boundaries without designing
a fine-grained capability system. Agents get stable coarse diagnostics for
repair, while the toolchain can still expose access-mode evidence for future
language-design decisions.
