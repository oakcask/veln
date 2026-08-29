---
role: proposal
update-when: The planned metrics behavior for invalid source-path module identities, partial report completeness, source diagnostics, policy evaluation, or baseline handling changes.
---

# Metrics Partial Source Analysis

## Summary

Allow `veln metrics` to return useful path-based measurements and the valid
portion of its module graph when a project-owned source has an invalid
source-path-derived module identity. The command must retain the direct source
diagnostic, omit only import diagnostics caused by removing that identity, and
mark the report incomplete. A partial graph cannot produce a successful policy
result because excluded identities can hide graph relationships.

This proposal changes the current fail-fast boundary in
[Metrics JSON](../specification/metrics-json.md). It does not make invalid
source code valid and does not weaken source diagnostics in other commands.

## Current Boundary

Metrics source discovery and source-graph validation currently fail on a
module identity error before returning a metrics report. Source-path casing is
specified by [Name Resolution](../specification/name-resolution.md). The
completed source-path module identity boundary deliberately excludes metrics
and dependency-cycle evidence; see
[Identifier Casing Source Path Module Identities](../reference/implemented-proposals/identifier-casing-source-path-module-identities.md).

## Proposed Result Contract

At least one `name.invalid_case` diagnostic with `origin: source_path` must be
present for a partial report. The command keeps every such diagnostic,
including its existing source association, span, details, and ordering.

Source graph analysis also reports `module.unresolved_import` after it rejects
an invalid source identity. Metrics omits that diagnostic only when the import
failure is a direct consequence of the same exclusion:

- a retained source imports the path-derived identity of an excluded source;
- an excluded source imports an identity whose ordinary project-relative
  `.veln` path exists among the project-owned sources.

These omitted diagnostics do not become report diagnostics. They do not make
the excluded identities available for resolution. An unresolved import to any
other identity, an import from an excluded source to a missing project path, or
any other error diagnostic keeps the current error envelope and returns no
report or policy result. This causal boundary is part of metrics behavior only;
other commands keep their current diagnostic sets.

A partial report has all ordinary report fields and these completeness fields:

- top-level `status` is `incomplete` unless a known policy violation takes
  precedence;
- top-level `diagnostics` contains the qualifying source diagnostics;
- top-level `completeness.status` is `partial`;
- top-level `completeness.excluded_sources` lists each project-relative source
  path excluded from the module graph with reason
  `invalid_module_identity`.

The JSON document keeps schema version `veln-metrics-json/v0`. Consumers must
branch on `status` and must not interpret the presence of report fields as
completeness.

The excluded-source list is sorted by normalized project-relative path. It
does not expose absolute paths or source text. Advisory partial analysis exits
nonzero even though report fields are present. This prevents automation from
treating an invalid project as a complete successful measurement.

In human mode, the command renders every qualifying diagnostic on the normal
diagnostic stream and renders the retained report on standard output. The
report begins with an incomplete-analysis notice and the excluded source
paths. The same nonzero exit rule applies to human and JSON output.

## Subject And Selection Boundaries

An invalid source-path-derived identity does not create a module record. Its
imports create no internal or external dependency edges. It cannot contribute
to a dependency cycle. Summary module and edge counts describe only the
retained valid graph.

ABC and whole-body similarity subjects are path-based. A parse-clean invalid
source remains eligible for those measurements when its path is selected.
These subjects keep their existing project-relative path identities and do not
acquire a module identity. A parse error keeps the current source-error gate
and prevents a partial report.

Explicit path selection remains visible in `project.selected_paths`, including
an explicitly selected invalid source. That path can select eligible path-based
subjects but cannot select a module record. An unselected invalid source still
appears in `completeness.excluded_sources` because metrics uses the containing
project graph for fan-in, fan-out, dependency pressure, and cycle membership.

## Policy And Baseline Boundaries

`metrics --check` evaluates `deny_cycles` on the retained valid graph:

- A known violation uses top-level `status: policy_violation`,
  `check.result: fail`, and a nonzero exit. The report still carries the
  diagnostics and partial-completeness fields.
- No known violation uses top-level `status: incomplete`,
  `check.result: incomplete`, an empty `check.violations` array, and a nonzero
  exit. It must not report `pass` because an excluded identity can hide a
  cycle.

A baseline comparison uses the retained valid graph. A known regression still
fails. A baseline subject excluded only because its current source identity is
invalid appears in `completeness.excluded_baseline_subjects`, sorted by module
identity, and does not appear in `check.baseline.stale_subjects`. The
completeness field is present only for baseline checks. In the absence of a
known regression, the check remains incomplete rather than passing.

`--write-baseline` refuses a partial report and does not create or replace the
requested baseline path. It must not persist incomplete graph data as a
complete future allowance.

## Acceptance Cases

The checked metrics cases are the primary planned evidence.

| Case | Input | Required observations |
| --- | --- | --- |
| Advisory partial report | One invalid source-path identity, one valid module that imports the would-be identity, and one unrelated valid source with an ABC subject. | The diagnostic is retained. The invalid source is listed as excluded and has no module or edge. The unrelated ABC subject remains. Status and completeness are incomplete and partial. The exit is nonzero. |
| Checked hidden cycle | Imports would form a cycle only if the invalid identity became a graph node. | The retained graph has no such cycle or violation. The check result is incomplete, not pass. The source diagnostic and excluded source remain visible. |
| Checked known cycle | Valid modules form a cycle while another source has an invalid identity. | The valid cycle produces a policy violation. Partial-completeness fields and the source diagnostic remain visible. |
| Explicit invalid selection | The command explicitly selects the invalid source and one valid source. | Both paths remain selected. Only the valid source has a module record. Eligible path-based subjects from both sources follow the subject boundary above. |
| Exclusion-caused import diagnostics | A retained source imports an excluded path-derived identity, and an excluded source imports an existing project-owned source. | The command omits only those resulting unresolved-import diagnostics, retains every qualifying casing diagnostic, and returns the partial report without creating recovery edges. |
| Mixed source errors | The project contains a qualifying source-path casing diagnostic and another parse error or unresolved import outside the causal boundary. | The command returns the current error envelope with no report, completeness object, or policy result. |
| Partial baseline write | A partial advisory report is requested with `--write-baseline`. | The command fails and does not create or replace the baseline path. |
| Partial baseline check | A baseline names a module whose current source identity is invalid. | The subject is excluded rather than stale. A known retained-graph regression fails; otherwise the result is incomplete. |

Focused metrics unit tests must isolate node creation, imports declared by an
identityless source, imports to an excluded identity, unrelated unresolved
imports on both sides of the causal boundary, path-based subject retention,
and the precedence between known policy violations and incomplete results.
JSON and human command cases must cover diagnostic retention and nonzero exit
behavior. A file-state test must cover baseline-write refusal without modifying
an existing path.

## Out Of Scope

- Retaining or broadly suppressing any source error other than source-path
  identifier casing. Only the two exclusion-caused unresolved-import shapes
  defined above may be omitted by metrics.
- Changing `check`, `test`, `run`, documentation, backend, package snapshot, or
  language-service source-error boundaries.
- Treating excluded identities as recovery modules or resolving imports
  through them.
- Suppressing, downgrading, relocating, or reordering the existing source-path
  diagnostic.
- Returning a successful advisory or policy result from a partial graph.

## Completion

Implementation is complete only when all acceptance cases have executable
evidence, the smallest metrics and command specification pages describe the
implemented schema and exit behavior, and the completed proposal record is
moved out of `docs/proposals/` by the proposal implementation audit workflow.

Updating the command specification must also retire the existing same-scope
`commands.md` and `commands-full.md` pair under the documentation authoring
policy. Because those files cover independently useful command subjects, the
migration must route their content into focused subject pages instead of
consolidating every command into one authority.

The implementation review must compare the existing generated similarity
workload before and after the change. Complete analysis must not acquire an
additional project-wide parse pass merely to identify sources that partial
analysis excludes; source diagnostics already establish the parse-clean
boundary.
