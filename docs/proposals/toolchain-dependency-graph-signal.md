# Toolchain Dependency Graph Signal

This proposal adds dependency-graph findings to the toolchain implementation
metrics signal. It is planned behavior, not current command behavior.

## Goal

Make crate-level coupling visible as a refactor signal alongside ABC
complexity and large-file findings. The signal should help reviewers decide
where a change needs boundary work, not block ordinary feature work by default.

## Research Basis

- `callo-arias-dependency-analysis-review`: dependency analysis is useful when
  it makes implementation interconnections explicit for maintenance and change
  impact work.
- `sangal-dependency-models`: dependency models and dependency-structure
  matrices support architecture management by turning extracted code
  relationships into reviewable structure.
- `sullivan-modularity-dsm`: design-structure matrices are a useful model for
  reasoning about information-hiding modularity and change value.
- `sarkar-modularization-metrics`: module-level dependency metrics are more
  useful than class-level metrics for large systems with package-like
  boundaries.
- `sas-architecture-smells`: cycles, hub-like dependencies, unstable
  dependencies, and oversized components are actionable architecture-smell
  families when reported with enough context to prioritize refactoring.
- `cargo-metadata`: Cargo already exposes workspace package metadata and a
  resolved dependency graph through machine-readable JSON.
- `github-workflow-commands`: GitHub annotations are line-oriented, while job
  summaries provide grouped Markdown for run-level evidence.
- `sarif`: static-analysis output should separate the common result envelope
  from tool-specific details.

## First Graph

Build the first graph from Cargo workspace metadata rather than Rust source
imports:

- Nodes are workspace crates.
- Edges point from the dependent crate to the workspace crate it depends on.
- Normal, dev, and build edges are retained as separate edge kinds.
- External registry dependencies are counted only as optional context, not as
  graph nodes in the first slice.
- The source location for a crate-level finding is the dependent crate manifest
  line that declares the edge. Whole-graph findings can point at the workspace
  manifest.

This fits the existing `veln-code-metrics` workflow because it is cheap,
stable, and available before module-level import analysis exists.

## Refactor Signals

Start with bounded signals that can be explained in one annotation:

- `dependency.cycle`: strongly connected workspace crates. Report the cycle
  members and the edge that closes the cycle.
- `dependency.hub`: a crate with both high incoming and high outgoing workspace
  degree relative to the current workspace median. Report in-degree,
  out-degree, and top adjacent crates.
- `dependency.unstable_dependency`: a crate with high incoming degree depending
  on crates with substantially higher instability. Use instability as
  `outgoing / (incoming + outgoing)` over normal workspace edges.
- `dependency.layer_drift`: optional configured layer order violation. Keep
  this disabled until the repository records an intended layer order.
- `dependency.hotspot`: a crate that is central in the workspace graph and also
  contains high ABC or large-file findings.

Do not treat a single high score as a required refactor. The message should say
what fact crossed the threshold and which local boundary to inspect.

## Output Shape

Keep annotations sparse and make summary the main aggregate view.

- Existing plain text output remains line-oriented for local runs.
- `--github-annotations` emits only the highest-ranked actionable findings,
  keeping the current truncation policy.
- Add `--github-summary` to append Markdown to `GITHUB_STEP_SUMMARY` when that
  environment variable is present.
- Add `--summary-path PATH` for CI systems that want a file without depending
  on GitHub environment variables.
- Consider `--json` later for tools, using a stable top-level envelope with
  finding-specific `details`.

Summary content should include:

- total workspace crates and workspace edges by kind
- whether cycles were found
- top crate-level findings by rank
- a compact table with crate, incoming, outgoing, instability, and signal
- annotation truncation count, when annotations omit findings

The summary is the right place for graph context because annotations are tied
to one file span and are capped for usability. The summary can show why a crate
is ranked even when no single edge explains the whole smell.

## CI Messages

Annotation headlines should state the failed fact. The body should state the
next useful action and why it matters.

Examples:

- `Dependency cycle`: `Break the cycle before adding new workspace edges;
  cycles make change impact and test selection harder to bound.`
- `Hub-like workspace crate`: `Inspect whether this crate owns too many
  directions of change; high incoming and outgoing degree makes review scope
  harder to isolate.`
- `Unstable dependency`: `Move the volatile dependency behind a smaller
  boundary or keep the edge justified; stable crates depending on volatile
  crates amplify future changes.`

## Implementation Route

1. Extend `veln-code-metrics` with a `Report` model so text, annotations, and
   summary render from the same ranked finding list.
2. Load workspace package metadata with `cargo metadata --format-version 1`.
   Prefer a dependency review before adding a Rust metadata parsing crate; the
   first implementation can shell out to Cargo and parse only the fields it
   needs.
3. Add graph construction and metric calculation behind a flag such as
   `--workspace-dependencies` before enabling it by default.
4. Add unit tests for graph metrics with synthetic crate graphs.
5. Add one toolchain-harness or workflow-script test that verifies summary
   rendering and annotation truncation.
6. Update `.github/workflows/signal--code-metrics.yaml` to request both
   annotations and summary after local output is stable.

## Open Questions

- Whether `veln-code-metrics` should depend on `cargo_metadata`, `serde_json`,
  or keep Cargo JSON parsing local and narrow.
- Whether dev-dependencies should contribute to instability, or only appear in
  summary context.
- Which layer order, if any, the repository wants to enforce for toolchain
  crates.
- Whether graph findings should stay warning-only or eventually become a
  separate quality gate.
