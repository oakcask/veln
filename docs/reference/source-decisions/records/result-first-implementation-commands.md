# Discussion Result: First Implementation Commands

Status: implemented

## Picked Question

- Which commands are required for the first implementation: `run`, `check`,
  `fmt`, `test`, `doc`, `graph`, `explain`, `repair`?

## Decision

The first implementation should provide four required commands: `veln check`,
`veln fmt`, `veln run`, and `veln test`.

`check` is the primary agent repair-loop command. `fmt` keeps generated and
human-edited source stable. `run` proves a selected entry point can execute
after static gates pass. `test` gives agents and humans one standard
verification command for examples and test files.

Delay `doc`, `graph`, `explain`, and `repair` until the core diagnostics are
stable enough for those commands to be thin views or workflows over existing
analysis results. The implemented `explain` command now follows that boundary
as a read-only diagnostic catalog.

## Rationale

The first command set should cover the smallest complete edit loop:
format source, check partial or complete source, execute complete code, and run
focused verification. These four commands are enough to test the language's
main claim that syntax, typed holes, diagnostics, and tools reduce repair cost.

Adding `doc`, `graph`, `explain`, or `repair` too early would expand the public
tool surface before the checker has stable enough data to explain. Those
commands are valuable, but they depend on diagnostics, module metadata,
dependency reachability, and repair-candidate semantics that are still open
design areas. Shipping them as first-slice commands would either freeze weak
interfaces or duplicate behavior that should come from `check`.

`test` belongs in the first implementation because a repair loop without a
standard verification command pushes agents back to project-specific scripts.
The command can start small: discover explicitly marked tests and examples,
run them after the same static gates used by `run`, and report results in a
machine-readable shape later aligned with `check`.

## First-Slice Rules

- `veln check` is required and remains the primary read-only diagnostic
  command.
- `veln fmt` is required and should preserve semantic tokens such as named hole
  labels while normalizing layout.
- `veln run <entry>` is required for executable entry points and must respect
  the hole runtime boundary before user code starts.
- `veln test` is required, but the first version may support only explicit test
  files and doctest-like examples that the parser already understands.
- `veln doc`, `veln graph`, and `veln repair` are not required first-slice
  commands.
- `veln explain` is optional in the first slice and, when present, must remain
  a read-only view over diagnostic catalog data.
- Deferred commands should consume the same analysis data as `check` when they
  are introduced rather than defining separate parsers or diagnostic formats.

## Open Detail

The exact first `test` discovery model can remain small. A direct file list,
`*_test.veln` convention, or explicit manifest field are all compatible with
this decision as long as agents can invoke one standard command.

The first JSON shape for `test` is resolved by
[Test JSON Shape](result-test-json-shape.md). It shares the stable-envelope
style used by diagnostics while keeping test result records separate from
diagnostic records.

## Consequence

The first implementation gets a complete standard loop without overcommitting
to higher-level explanation and repair workflows. Agents can depend on
`fmt`, `check`, `run`, and `test` while the language continues to refine module
metadata, dependency graphs, and repair candidates.
