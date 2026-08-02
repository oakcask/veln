---
review-when: The metrics scope, acceptance evidence, enforcement graduation criteria, or implementation status changes.
---

# Built-In Code Metrics

Status: proposed

## Summary

Add a language-aware `veln metrics` command for Veln source. The command reports
function ABC size vectors, internal module dependency cycles, module fan-in and
fan-out, dependency pressure, and exact whole-body similarity groups.

The command separates measurement from enforcement. A normal invocation is an
advisory report. The first implementation slice permits `veln metrics --check`
to reject only new or worsened dependency cycles. ABC, dependency counts,
dependency pressure, and similarity remain advisory until project evidence
meets the enforcement graduation criteria in this proposal.

## Motivation

AI-generated changes can add locally plausible code while gradually increasing
function size, ownership pressure, dependency cycles, and repeated logic. These
properties cross file boundaries and are easy to miss when a review focuses on
functional output.

This repository already uses `veln-code-metrics` to guide Rust refactoring. It
reports Rust function ABC values, file dependencies, dependency pressure, and
dependency cycles. Repository experience supports dependency cycles as a
blocking ownership signal. ABC and dependency pressure remain review signals;
their current thresholds have not been validated as portable merge gates.

The existing executable parses Rust with `syn`. It does not analyze Veln source
and is not part of the installed Veln CLI.

Veln already owns the source parser, module identities, `use` declarations,
package discovery, and source spans needed for a more accurate analysis. A
built-in command can distinguish source imports from the implicit standard
prelude, keep package boundaries explicit, and return locations that agents and
editors can act on.

## Decision

Add a dedicated command instead of adding metric warnings to `veln check`.

`veln check` answers whether source satisfies the language rules. Code metrics
are maintainability signals whose useful limits vary by project. Keeping the
commands separate prevents a quality policy from appearing to be a language
error and lets CI opt into reviewed enforcement explicitly.

Do not generalize the existing Rust-only executable into the public command.
The two tools have different syntax and module models. They may share
presentation conventions, but Veln metrics must operate on Veln syntax and
project analysis artifacts.

The first slice uses differentiated policy maturity:

| Signal | First-slice role | Reason |
| --- | --- | --- |
| Dependency cycles | Advisory and enforceable | A cycle is a concrete bidirectional ownership constraint, and this repository has used the signal to remove real cycles before enabling a gate |
| ABC size | Advisory | ABC measures syntactic size rather than maintainability or comprehension difficulty directly |
| Fan-in | Advisory | High fan-in can identify a stable and intentionally reused module as well as a large change-impact surface |
| Fan-out | Advisory | High fan-out can identify coordination cost, but no Veln-specific blocking threshold has acceptance evidence |
| Dependency pressure | Advisory | The product of fan-in and fan-out is useful for triage but is not an architectural verdict |
| Exact whole-body similarity | Experimental advisory | Exact matching favors precision, but usefulness and repair intent require project acceptance data |

## Command Surface

The proposed command forms are:

```text
veln metrics [--json] [path ...]
veln metrics --check [--json] [--baseline PATH] [path ...]
veln metrics --write-baseline PATH [path ...]
```

Path discovery follows `veln check`. If no path is provided, the command
discovers the current project.

A path selection limits which project-owned modules and declarations appear as
subjects in the report. The command still resolves the complete containing
project graph when that graph is required to calculate a selected module's
fan-in, fan-out, pressure, or cycle membership.

Dependency packages and embedded standard-library modules are excluded as
metric subjects in the first slice. An import from a selected module to a
dependency package is reported as an external dependency count. It does not
create an internal graph edge.

Generated project modules remain graph nodes because project-owned source can
depend on them. Generated and doctest-derived declarations are excluded from
ABC and similarity subjects. JSON identifies generated graph nodes so a
consumer does not mistake them for hand-maintained modules.

## Modes And Exit Status

| Invocation | Policy behavior | Successful exit condition |
| --- | --- | --- |
| `veln metrics` | Report only | Analysis completed, even when findings exist |
| `veln metrics --check` without a baseline | Apply enabled first-slice policy | Analysis completed, at least one policy is enabled, and no enabled policy was violated |
| `veln metrics --check --baseline PATH` | Apply enabled policy and cycle regression rules | Analysis completed, at least one policy is enabled, and no policy regression was found |
| `veln metrics --write-baseline PATH` | Write the complete current report as a baseline | Analysis completed and the baseline was written atomically |

The first slice has one enforceable policy: `deny_cycles`. A check with no
enforceable policy enabled is a configuration error. Advisory thresholds must
not silently become merge gates through `--check`.

Invalid command arguments use the existing CLI usage-error status. Source
discovery, parsing, module identity, import resolution, manifest, or metric
analysis errors make every mode fail. Type, effect, and contract errors do not
prevent metric analysis and are not metric-command diagnostics. A partial
metric report must not be presented as a clean result.

`--baseline` is valid only with `--check`. `--write-baseline` conflicts with
`--check`, `--json`, and an existing target file unless an explicit replacement
option is added in a later proposal. These conflicts prevent an automated check
from silently accepting its own regression.

## Project Policy

The command reads string-valued fields from `[tool.metrics]` in `veln.toml`.
The first slice recognizes these fields:

```toml
[tool.metrics]
deny_cycles = "true"
similarity_min_tokens = "60"
max_findings = "50"
```

Every field is optional. `similarity_min_tokens` defaults to `60`,
`max_findings` defaults to `50`, and `deny_cycles` defaults to `false`.
`similarity_min_tokens` and `max_findings` must be positive integers.
Unknown fields and invalid values are command errors with a span on the
manifest field.

`similarity_min_tokens` controls an experimental advisory signal. It does not
become an enforcement threshold in the first slice. `max_findings` limits
detailed human-output entries after sorting. It does not change collection,
JSON output, baseline content, policy evaluation, or the summary. Truncated
human output states how many findings were omitted and identifies `--json` as
the complete evidence route.

The command-line baseline path overrides a future manifest baseline field. The
first slice does not load a baseline implicitly. CI and local automation must
name the reviewed file they intend to enforce.

## Metric Model

### Function ABC Size

For each eligible Veln function and test, the command reports the vector
`(A, B, C)` and its magnitude `sqrt(A^2 + B^2 + C^2)`. The magnitude is rounded
to one decimal place for display. JSON also includes the unrounded value and
the subject kind `function` or `test`.

ABC is a syntactic size measure. The command must not describe an ABC value as
proof that a function is complex, defective, or poorly designed.

The Veln mapping is:

| Component | Counted source construct |
| --- | --- |
| Assignment | Each `let` body line |
| Branch | Each call, effect performance, handler application, schema decode, or schema encode expression |
| Conditional | Each `if` condition, each `else if` condition, each `match` expression, each match arm, each short-circuit `and` or `or`, and each `?` expression |

Parameters, result bindings, type annotations, effect annotations, and contract
text are not assignments. Nested expressions contribute to the containing
function. Declaration signatures do not contribute.

Contracts are excluded until their predicates have the same structured AST
coverage as function expressions. Human and JSON output state that contracts
are excluded. JSON includes `contracts_included: false` for every affected
subject.

### Module Dependency Graph

Each project-owned Veln module identity is a node. Each source-written `use`
from one project-owned module to another is one directed edge. Duplicate imports
between the same pair of modules produce one edge. Implicit standard-prelude
imports do not produce edges.

Fan-in is the number of distinct project-owned modules with an edge to the
subject. Fan-out is the number of distinct project-owned modules to which the
subject has an edge. External dependency count is reported separately and does
not contribute to either value.

Dependency pressure is `fan-in * fan-out`. Output always presents pressure with
both component counts. Pressure is a triage order, not a policy violation.

A dependency cycle is a maximal strongly connected group containing more than
one module. A self-import is also a cycle if source analysis permits it to
reach metric analysis. The report gives every member and at least one concrete
closed edge path for each cycle.

### Exact Whole-Body Similarity

The first slice compares complete bodies of eligible project-owned functions
and tests. Comments, documentation text, whitespace, and formatting do not
affect similarity. Identifier spelling and literal values remain significant.

A similarity instance is an equivalence class containing two or more
declarations whose complete normalized body token sequences are equal and
contain at least `similarity_min_tokens` tokens. Each declaration appears in at
most one instance. The command does not compare arbitrary subregions and does
not report repeated regions within one declaration.

An instance identity contains the normalized token sequence and the sorted
canonical declaration identities. It excludes byte offsets and formatting.
Moving an unchanged declaration within its source file does not change the
instance identity.

Similarity is report-only in the first slice. A changed token sequence creates
a new advisory instance instead of a policy regression. This rule prevents a
coordinated improvement to duplicate bodies from being rejected only because
the evidence fingerprint changed.

Partial-body, identifier-insensitive, approximate, or semantic clone detection
is a non-goal for the first slice. These modes require separate precision,
recall, output-growth, and maintainer-acceptance evidence.

## Baseline And Regression Rules

A baseline is a versioned JSON document produced by the same metric report
schema. It records metric-model identity, effective configuration,
project-relative subjects, raw metric values, dependency edges, cycle
membership, and advisory similarity instances. It does not contain absolute
paths or source text.

The first slice uses a baseline only for dependency-cycle policy:

| Current graph change | `deny_cycles = "true"` without baseline | `deny_cycles = "true"` with baseline |
| --- | --- | --- |
| No cycle | Pass | Pass |
| Unchanged baseline cycle | Fail | Pass |
| Baseline cycle loses a member or cyclic edge | Fail if a cycle remains | Pass |
| Current cycle adds a member or cyclic edge not contained in one baseline cycle | Fail | Fail |
| New self-cycle | Fail | Fail |

A renamed module does not inherit an unrelated module identity. If a rename
leaves a cycle under a new identity, the check fails and exposes the current
closed path. Deleting a subject deletes its baseline allowance. A baseline
entry that no longer resolves is reported as stale but does not by itself fail
the check.

ABC, fan-in, fan-out, pressure, and similarity values remain visible in a
baseline for review and future migration. They do not affect first-slice exit
status. A later proposal must not reinterpret an existing baseline as an
allowance for a newly enforceable metric without a schema change and an
explicit baseline rewrite.

The baseline schema rejects unsupported schema or metric-model versions.
Comparison is deterministic across operating systems after project-relative
paths are normalized with `/` separators.

## Enforcement Graduation

A later proposal may make an advisory metric enforceable only after it provides
project evidence for the exact subject population, metric mapping, threshold,
and diagnostic text that will become policy.

The evidence set must satisfy all of these conditions:

1. It analyzes representative hand-maintained Veln projects and separates
   functions, tests, and generated subjects.
2. A maintainer labels every finding when the set contains at most 100
   findings. For a larger set, the maintainer labels a deterministic sample of
   100 findings.
3. Each label is `action-required`, `advisory-useful`, or `not-useful`, with a
   short reason.
4. Fewer than 10 percent of the reviewed findings are `not-useful` before the
   metric can block a check.
5. The proposed blocking condition rejects a known worsened case and passes an
   equal case, an improved case, a subject rename that preserves the intended
   allowance, and a relevant subject-kind boundary.
6. The proposal records how the threshold was derived from project evidence.
   It must not adopt the Rust tool's threshold or a literature value without
   Veln-specific validation.
7. The human diagnostic identifies the measured fact and a concrete review
   action. It must not claim that a metric alone proves a defect.

The evidence belongs in a review record, not in the current behavior
specification. A metric that misses any graduation condition remains advisory.

## Human Output

Human output starts with a summary and then emits findings in this order:

1. policy violations;
2. dependency cycles;
3. modules by descending dependency pressure, then fan-out and fan-in;
4. functions by descending ABC magnitude;
5. whole-body similarity instances by descending token count.

Ties use project-relative path, starting byte offset, and finding kind. Output
states whether the invocation is report-only or checked against cycle policy
and a baseline.

The primary line names the measured fact at its source span. Explanations,
baseline differences, and other locations use related notes. Similarity
findings use one primary declaration and related notes for the other
declarations.

A cycle violation states the enabled policy, identifies a concrete closed
path, and tells the maintainer to inspect module ownership and dependency
direction. The final failure summary identifies `veln metrics --json` as the
complete evidence when human output was truncated. It must not tell automation
to update the baseline as the default repair.

Advisory ABC output calls the measurement `ABC size`. Dependency output labels
fan-in, fan-out, and pressure separately. Similarity output labels the signal
`experimental` and does not instruct the maintainer to deduplicate code
mechanically.

## JSON Output

`veln metrics --json` emits one JSON document. Executable CLI fixtures and the
metrics section added to the JSON specification will become authoritative
during implementation.

The document contains:

- `schema`, `metric_model`, `tool`, `status`, and normalized project identity;
- analysis diagnostics;
- effective configuration, enforceable policy capabilities, and baseline
  identity;
- per-function ABC vectors, magnitude, subject kind, span, and coverage flags;
- per-module internal fan-in, internal fan-out, dependency pressure, external
  dependency count, generated status, and span;
- dependency edges and cycles;
- experimental whole-body similarity instances with token count, fingerprint,
  and declaration regions;
- policy violations and a summary by metric kind.

Arrays have the same canonical order on repeated runs. Numeric values are JSON
numbers, not formatted strings. Project-relative paths never become absolute
paths in the document.

## Acceptance Cases

Planned executable cases follow the placement rules in
`../../examples/specification/README.md` and use capability-specific case names.

| Case | Input distinction | Required observation |
| --- | --- | --- |
| ABC constructs | One function uses every counted construct and one changes only annotations or contracts | The vector follows the mapping table, the annotation-only change does not alter it, and output calls it ABC size |
| ABC subject kinds | Equivalent function and test bodies are analyzed; a generated declaration has the same body | Function and test results carry distinct subject kinds; the generated declaration is excluded |
| Graph counts | Modules contain repeated internal imports, an external package import, and an implicit prelude | Internal edges are deduplicated and external or implicit imports do not change fan-in or fan-out |
| Dependency pressure | Modules have high fan-in only, high fan-out only, and both | Pressure equals the product, output retains both counts, and none is a policy violation |
| Dependency cycle | Three modules form a cycle and one acyclic module imports a member | One maximal cycle and a valid closed path are reported; the acyclic caller only changes fan-in |
| Exact whole-body similarity | Two formatted-differently bodies have the same tokens; a third changes an identifier | The first pair forms one experimental instance and the third is not included |
| Partial similarity exclusion | Two declarations share a long subregion but have different complete bodies | No similarity instance is reported |
| Similarity edit | Two duplicate bodies are changed together to another equal token sequence | The new instance is advisory and does not fail a baseline check |
| Similarity result bound | Many declarations contain the same body and many unrelated bodies | Each declaration appears at most once, total reported regions do not exceed eligible declarations, and instances do not exceed half the eligible declarations |
| Stable ordering | Discovery order and path separator representation vary | Normalized JSON findings and fingerprints are identical |
| Advisory mode | Advisory findings are present and a cycle exists | Report mode exits successfully and labels the findings advisory |
| Check without policy | `--check` runs with `deny_cycles` omitted or false | The command reports that no enforceable policy is enabled and fails without a clean check result |
| Absolute cycle check | A cycle exists with `deny_cycles` enabled and no baseline | Check mode fails and reports a closed path |
| Cycle baseline regression | A baseline cycle stays equal, loses an edge, and gains an edge in separate runs | Equal and improved reports pass; the worsened report fails |
| Stale baseline | A baseline subject is deleted | The command reports the stale entry without failing solely for staleness |
| Unsupported metric model | Baseline and current reports use different metric-model versions | Comparison fails without treating advisory records as allowances |
| Invalid source | One selected module does not parse | The command fails and does not emit a clean metric summary |
| Invalid types | Selected source parses but has a type error | Metrics are reported without presenting the type error as a metric diagnostic |
| Baseline write safety | The target already exists | Baseline writing refuses to replace it |
| Truncated human output | Findings exceed `max_findings` | Policy uses the complete set; human output names the omitted count and the JSON evidence command |

CLI parsing, human output, JSON shape, exit status, and atomic baseline writes
must have integration coverage in `veln-cli`. Metric calculation and baseline
comparison must have table-driven unit coverage in a reusable metrics library.

## Bounded Analysis Requirement

Similarity analysis is limited to one normalized token sequence per eligible
declaration. This internal constraint is normative because it bounds the
observable result set and prevents an unrestricted source-region comparison.

For `N` eligible declarations:

- the command creates exactly `N` declaration fingerprints;
- each declaration contributes to at most one similarity instance;
- the total number of reported similarity regions is at most `N`;
- the number of reported similarity instances is at most `floor(N / 2)`.

Normal CI includes a structural test for these bounds with unrelated bodies,
one large equivalence class, and many two-declaration equivalence classes.

A controlled generated benchmark contains unrelated functions, repeated
functions, and repeated token prefixes at three adjacent sizes. It reports wall
time, user CPU time, peak resident memory, source token count, declaration
fingerprint count, similarity instance count, and reported region count.
Doubling unrelated input tokens must not increase median user CPU time or peak
resident memory by more than three times between adjacent sizes on the same
machine and build profile.

The benchmark is review evidence, not a portable CI time limit. The structural
bounds are the authoritative normal-CI guard.

## Implementation Guidance

This section is not normative except where the bounded-analysis requirement
constrains the implementation result.

Create a reusable Veln metrics library rather than placing calculation in the
CLI command module. Reuse `veln-project` discovery and the lowered surface AST.
Preserve per-source module identity before merged project analysis discards
file grouping.

Use source `UseOrigin` to exclude implicit prelude edges. Compute graph values
from canonical module identities rather than textual aliases. Derive
similarity tokens from the Veln lexer so formatting and comments do not become
semantic input.

An implementation can group complete body fingerprints to keep similarity
analysis bounded. It must verify candidate declarations against their complete
normalized token sequences before reporting equality. A fingerprint collision
must not create a finding.

The existing Rust-only `veln-code-metrics` remains a repository maintenance
tool. Do not make the public command depend on `syn` or accept Rust source.

## Non-Goals

- Do not change Veln syntax, typing, effect, contract, or runtime semantics.
- Do not make metric limits part of `veln check`.
- Do not claim that one threshold is universally healthy for every project.
- Do not describe ABC as a direct complexity or defect measure.
- Do not enforce ABC, fan-in, fan-out, pressure, or similarity in the first
  slice.
- Do not automatically rewrite functions, split modules, or deduplicate code.
- Do not automatically update a baseline during `--check`.
- Do not measure dependencies inside external packages in the first slice.
- Do not include generated or doctest-derived declarations in ABC or
  similarity analysis.
- Do not add partial-body, approximate, identifier-insensitive, or semantic
  clone detection in the first slice.
- Do not use a single combined maintainability score that hides the underlying
  measurements.

## Planned Verification Commands

Implementation must make these repository-relative checks available:

```sh
bash scripts/agent-test -p veln-metrics
bash scripts/agent-test -p veln-cli --test toolchain_harness
bash scripts/agent-run cargo run --locked -p veln-cli -- metrics --json path/to/project
bash scripts/benchmark-veln-metrics compare SMALL MEDIUM LARGE
```

The metrics crate, executable cases, and benchmark command do not exist until
this proposal is implemented.

## Completion Boundary

This proposal is complete only when all acceptance cases pass, the generated
structural guard runs in normal CI, the controlled benchmark meets the
bounded-analysis requirement, and the cycle policy has repository review
evidence. Advisory metrics do not need enforcement graduation for this first
slice to complete.

Completion must add current command behavior to
`../specification/commands-full.md`, route it from
`../specification/commands.md`, specify JSON behavior through
`../specification/json-output.md`, and place named executable cases according
to `../../examples/specification/README.md`.

After completion, move this document to
`../reference/implemented-proposals/` and remove it from the proposal catalog.
