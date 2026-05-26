# Discussion Result: Minimal Project and Test Discovery

Status: implemented

## Picked Question

- What minimal project, import, entry-point, and test discovery model should
  `check`, `run`, and `test` share before package manifests and `graph` exist?

## Decision

The first implementation should use one shared project context for
`veln check`, `veln run`, and `veln test`.

That context starts from explicit command-line targets. If a command receives a
file, directory, entry point, or test target, that target bounds discovery. If
no target is provided, the current directory is the project root and all
source-relative `*.veln` files below it form the discovered source set. A
single file with no cross-file imports remains a valid implicit project and
does not require a manifest.

Before a concrete package manifest format exists, imports resolve only inside
the discovered source set. Source files with `mod` declarations use those
module identities; files without module declarations get source-relative
implicit module identities only for diagnostics and single-file execution.
External package dependencies, workspace membership, and non-default source
roots remain manifest-owned future work rather than hidden command behavior.

Entry points must be explicit for `run` unless exactly one executable public
`main`-shaped function is discovered. Test discovery starts with explicit test
targets, source-declared test cases, same-file executable examples, and
source-relative `*_test.veln` files as an organization convention. Automatic
narrowing is allowed only when this evidence is complete for the discovered
source set; otherwise `test` widens and reports the missing evidence. The
source spelling for user-authored test cases is resolved by
[Test Declaration Syntax](result-test-declaration-syntax.md).

## Rationale

The shared context keeps the first toolchain from creating three subtly
different notions of "the project." Agents should not see a program pass
`check`, fail `run` because entry resolution used another root, then miss tests
because `test` discovered files through a third convention. One source set,
one import resolver, and one target model are enough for the first repair loop.

The manifest and module decisions already split package-scale facts from
source-level semantic facts. Parnas supports keeping module boundary knowledge
near the code, while DeRemer and Kron distinguish programming small source
units from programming large compositions. For the first implementation, this
means source-level imports can be checked immediately, but package dependency
resolution, workspaces, publishing metadata, and source-root customization
should wait for the manifest format instead of being inferred from ad hoc
directory conventions.

Dependency-network research reinforces that package metadata is useful but
not enough to understand actual source use. Prazi argues for combining package
information with source or call evidence. Veln should therefore let the first
implementation build a small source-backed graph from parsed imports, but it
should not pretend to have a complete package graph before the manifest and
`graph` command exist.

Regression-test-selection research gives the safety rule for `veln test`.
Rothermel and Harrold define safe selection around not excluding tests that
could reveal faults under stated conditions. Their later empirical work shows
that safe selection can reduce cost, but its benefit depends on test-suite and
program factors. Graves, Harrold, Kim, Porter, and Rothermel compare RTS
techniques and emphasize practical cost-benefit tradeoffs. For Veln, this
supports conservative widening: a fast narrowed run is useful only when the
tool can state why the selected tests cover the edited scope.

## First-Slice Rules

- `check`, `run`, and `test` construct the same project context before their
  command-specific phase starts.
- Command-line files, directories, entry points, and test targets are trusted
  explicit bounds. With no target, the current directory is the project root.
- A source set is all explicitly named `*.veln` files, all `*.veln` files below
  explicitly named directories, or all `*.veln` files below the default root.
- A single-file program with no cross-file imports needs no manifest and no
  explicit `mod` declaration.
- Cross-file imports resolve only to modules in the discovered source set
  until a manifest-owned package dependency model exists.
- Source-relative paths in diagnostics must stay relative to the project root
  or to the explicit single-file target's containing directory.
- `run` requires an explicit entry point unless exactly one executable public
  `main`-shaped function is discovered.
- `test` discovers explicit test targets first, then source-declared test
  cases, then same-file executable examples, then source-relative `*_test.veln`
  files as an organization convention.
- Automatic affected-test narrowing must reuse the discovered source set and
  import evidence. If evidence is incomplete, `test` widens to all discovered
  tests and reports `selection_confidence: "partial"` or `"unknown"`.
- The first implementation should not infer package roots from hidden files,
  generated directories, lockfiles, or host-language project markers.

## Open Detail

The exact spelling of an entry-point target can remain provisional. A compact
form such as `path/to/file.veln:main` or `module.name:main` is compatible with
this decision as long as all commands normalize it into the same project
context.

The first manifest file name and source-root table remain separate decisions.
When they arrive, they should extend the shared context builder rather than
adding command-specific discovery paths.

The first JSON shape for `test` is resolved by
[Test JSON Shape](result-test-json-shape.md). This decision only requires that
selected targets, discovery evidence, widening reasons, and selection
confidence are visible.

## References

- Parnas, D. L. (1972). On the Criteria To Be Used in Decomposing Systems
  into Modules. *Communications of the ACM*, 15(12), 1053-1058.
  https://doi.org/10.1145/361598.361623
- DeRemer, F., & Kron, H. H. (1976). Programming-in-the-Large Versus
  Programming-in-the-Small. *IEEE Transactions on Software Engineering*, 2(2),
  80-86. https://doi.org/10.1109/TSE.1976.233534
- Hejderup, J., Beller, M., Triantafyllou, K., & Gousios, G. (2021).
  *Prazi: From Package-based to Call-based Dependency Networks*.
  arXiv:2101.09563. https://arxiv.org/abs/2101.09563
- Rothermel, G., & Harrold, M. J. (1997). A safe, efficient regression test
  selection technique. *ACM Transactions on Software Engineering and
  Methodology*, 6(2), 173-210. https://doi.org/10.1145/248233.248262
- Rothermel, G., & Harrold, M. J. (1998). Empirical studies of a safe
  regression test selection technique. *IEEE Transactions on Software
  Engineering*, 24(6), 401-419. https://doi.org/10.1109/32.689399
- Graves, T. L., Harrold, M. J., Kim, J.-M., Porter, A., & Rothermel, G.
  (2001). An empirical study of regression test selection techniques.
  *ACM Transactions on Software Engineering and Methodology*, 10(2), 184-208.
  https://doi.org/10.1145/367008.367020

## Consequence

The first implementation gets a small, auditable execution model: one
source-backed project context, local-only imports, explicit entry points, and
conservative test discovery. This keeps single-file examples cheap while
making cross-file and test behavior visible enough for agents to trust or
challenge the tool's result.
