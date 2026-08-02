# Built-In Code Metrics

Status: proposed

## Summary

Add a language-aware `veln metrics` command for Veln source. The command reports
function ABC metrics, internal module dependency cycles, module fan-in and
fan-out, and code-similarity candidates.

The command separates measurement from enforcement. A normal invocation is an
advisory report. `veln metrics --check` applies project policy and can reject
new or worsened findings relative to a reviewed baseline.

## Motivation

AI-generated changes can add locally plausible code while gradually increasing
function complexity, coupling, dependency cycles, and repeated logic. These
properties cross file boundaries and are easy to miss when a review focuses on
functional output.

This repository already uses `veln-code-metrics` to guide Rust refactoring. It
measures Rust function ABC values, file dependencies, fan-in, fan-out, and
dependency cycles. That executable parses Rust with `syn`. It does not analyze
Veln source and is not part of the installed Veln CLI.

Veln already owns the source parser, module identities, `use` declarations,
package discovery, and source spans needed for a more accurate analysis. A
built-in command can therefore distinguish source imports from the implicit
standard prelude, keep package boundaries explicit, and return locations that
agents and editors can act on.

## Decision

Add a dedicated command instead of adding metric warnings to `veln check`.

`veln check` answers whether source satisfies the language rules. Code metrics
are maintainability signals whose useful limits vary by project. Keeping the
commands separate prevents a quality policy from appearing to be a language
error and lets CI opt into enforcement explicitly.

Do not generalize the existing Rust-only executable into the public command.
The two tools have different syntax and module models. They may share
presentation conventions, but Veln metrics must operate on Veln's syntax and
project analysis artifacts.

## Command Surface

The proposed command forms are:

```text
veln metrics [--json] [--baseline PATH] [path ...]
veln metrics --check [--json] [--baseline PATH] [path ...]
veln metrics --write-baseline PATH [path ...]
```

Path discovery follows `veln check`. If no path is provided, the command
discovers the current project.

A path selection limits which project-owned modules and functions appear as
subjects in the report. The command still resolves the complete containing
project graph when that graph is required to calculate a selected module's
fan-in, fan-out, or cycle membership.

Dependency packages and embedded standard-library modules are excluded as
metric subjects in the first implementation slice. An import from a selected
module to a dependency package is reported as an external dependency count. It
does not create an internal graph edge.

## Modes And Exit Status

| Invocation | Policy behavior | Successful exit condition |
| --- | --- | --- |
| `veln metrics` | Report only | Analysis completed, even when findings exist |
| `veln metrics --check` without a baseline | Apply configured absolute limits | Analysis completed and no configured limit was violated |
| `veln metrics --check --baseline PATH` | Apply configured absolute limits and baseline regression rules | Analysis completed and no policy violation was found |
| `veln metrics --write-baseline PATH` | Write the complete current report as a baseline | Analysis completed and the baseline was written atomically |

Invalid command arguments use the existing CLI usage-error status. Source
discovery, parsing, module identity, import resolution, manifest, or metric
analysis errors make every mode fail. Type, effect, and contract errors do not
prevent metric analysis and are not metric-command diagnostics. A partial
metric report must not be presented as a clean result.

`--write-baseline` conflicts with `--check`, `--json`, and an existing target
file unless an explicit replacement option is added in a later proposal. This
prevents an automated check from silently accepting its own regressions.

## Project Policy

The command reads string-valued fields from `[tool.metrics]` in `veln.toml`.
The first slice recognizes these fields:

```toml
[tool.metrics]
abc_max = "30"
fan_in_max = "20"
fan_out_max = "12"
deny_cycles = "true"
similarity_min_tokens = "60"
similarity_max_instances = "0"
max_findings = "50"
```

Every field is optional. `similarity_min_tokens` defaults to `60`,
`max_findings` defaults to `50`, and `deny_cycles` defaults to `false`. An
omitted enforcement limit remains report-only. Numeric limits are non-negative
integers except that `similarity_min_tokens` and `max_findings` must be
positive. `abc_max` is a positive finite number. Unknown fields and invalid
values are command errors with a span on the manifest field.

`max_findings` limits detailed human-output entries after sorting. It does not
change collection, JSON output, baseline content, policy evaluation, or the
summary. Truncated human output states how many findings were omitted and tells
the maintainer to inspect `--json` before changing policy.

The command-line baseline path overrides a future manifest baseline field. The
first slice does not load a baseline implicitly. CI and local automation must
name the reviewed file they intend to enforce.

## Metric Model

### Function ABC

For each Veln function and test, the command reports the vector `(A, B, C)` and
its magnitude `sqrt(A^2 + B^2 + C^2)`. The magnitude is rounded to one decimal
place for display. Policy comparison uses the unrounded value.

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
coverage as function expressions. The report includes
`contracts_included: false` so a consumer cannot assume full-function coverage.

### Module Dependency Graph

Each project-owned Veln module identity is a node. Each source-written `use`
from one project-owned module to another is one directed edge. Duplicate imports
between the same pair of modules produce one edge. Implicit standard-prelude
imports do not produce edges.

Fan-in is the number of distinct project-owned modules with an edge to the
subject. Fan-out is the number of distinct project-owned modules to which the
subject has an edge. External dependency count is reported separately and does
not contribute to either value.

A dependency cycle is a maximal strongly connected group containing more than
one module. A self-import is also a cycle if source analysis permits it to
reach metric analysis. The report gives every member and at least one concrete
closed edge path for each cycle.

### Code Similarity

The first slice reports token-based similarity between project-owned function
and test bodies. Comments, documentation text, whitespace, and formatting do
not affect similarity. Identifier spelling and literal values remain
significant. Generated sources and doctest-derived sources are excluded.

A similarity instance contains two or more non-overlapping source regions with
the same normalized token sequence and at least `similarity_min_tokens` tokens.
The command reports maximal instances. It does not also report a shorter
instance fully contained at the same locations.

The same region pair appears once. Its canonical identity is independent of
file discovery order. Repeated syntax within one function can be reported when
the regions do not overlap.

Identifier-insensitive or approximate similarity is a non-goal for the first
slice. Such modes increase recall but also increase false positives. They need
separate acceptance data before they can become an enforcement signal.

## Baseline And Regression Rules

A baseline is a versioned JSON document produced by the same metric report
schema. It records tool schema identity, project-relative subjects, raw metric
values, dependency edges, cycle membership, and similarity instance
fingerprints. It does not contain absolute paths or source text.

With `--check --baseline PATH`, the following changes are regressions:

| Metric | Regression condition |
| --- | --- |
| ABC | A function newly exceeds `abc_max`, or an already over-limit function's unrounded magnitude increases |
| Fan-in | A module newly exceeds `fan_in_max`, or an already over-limit module's fan-in increases |
| Fan-out | A module newly exceeds `fan_out_max`, or an already over-limit module's fan-out increases |
| Cycles | `deny_cycles` is true and a current cycle adds a member or cyclic edge that is not contained in one baseline cycle |
| Similarity | An instance at or above `similarity_min_tokens` has no equivalent baseline fingerprint, or a baseline instance gains tokens or subject locations |

A rename does not inherit an unrelated subject's allowance. Deleting a subject
deletes its baseline allowance. A baseline entry that no longer resolves is
reported as stale but does not by itself fail the check.

For similarity, a baseline fingerprint includes the normalized token sequence
and canonical module and declaration identities. It excludes byte offsets and
formatting, so moving an unchanged declaration within its source file does not
create a regression.

Absolute configured limits still apply to subjects absent from the baseline.
Without a baseline, `similarity_max_instances` limits the total number of
reported instances. With a baseline, the similarity regression rule above
allows unchanged existing instances even when their count exceeds that limit.
The baseline cannot suppress source or manifest errors.

The baseline schema must reject unsupported schema versions. Comparison must
be deterministic across operating systems after project-relative paths are
normalized with `/` separators.

## Human Output

Human output starts with a summary and then emits findings in this order:

1. policy violations;
2. dependency cycles;
3. functions by descending ABC magnitude;
4. modules by descending fan-out, then fan-in;
5. similarity instances by descending token count.

Ties use project-relative path, starting byte offset, and finding kind. Output
states whether it is report-only or checked against policy and a baseline.

The primary line names the measured fact at its source span. Explanations,
baseline differences, and other locations use related notes. Similarity
findings use one primary region and related notes for the other regions.

Each policy-violation entry states the configured or baseline limit, tells the
maintainer to inspect the named subject and related evidence, and explains that
the check prevents new complexity, coupling, cycles, or duplicated logic from
becoming the next baseline. The final failure summary names `veln metrics
--json` as the complete evidence when human output was truncated. It must not
tell automation to update the baseline as the default repair.

## JSON Output

`veln metrics --json` emits one JSON document. The authoritative schema will be
specified by executable CLI fixtures and the metrics section added to the JSON
specification during implementation.

The document contains:

- `schema`, `tool`, `status`, and normalized project identity;
- analysis diagnostics;
- effective configuration and baseline identity;
- per-function ABC vectors, magnitude, span, and coverage flags;
- per-module internal fan-in, internal fan-out, external dependency count, and
  span;
- dependency edges and cycles;
- similarity instances with token count, fingerprint, and all source regions;
- policy violations and a summary by metric kind.

Arrays have the same canonical order on repeated runs. Numeric values are JSON
numbers, not formatted strings. Project-relative paths never become absolute
paths in the document.

## Acceptance Cases

Planned executable cases belong under `examples/specification/metrics/`.

| Case | Input distinction | Required observation |
| --- | --- | --- |
| ABC constructs | One function uses every counted construct and one changes only annotations or contracts | The vector follows the mapping table and the annotation-only change does not alter it |
| Graph counts | Modules contain repeated internal imports, an external package import, and an implicit prelude | Internal edges are deduplicated and external or implicit imports do not change fan-in or fan-out |
| Dependency cycle | Three modules form a cycle and one acyclic module imports a member | One maximal cycle and a valid closed path are reported; the acyclic caller only changes fan-in |
| Exact similarity | Two formatted-differently bodies have the same tokens; a third changes an identifier | The first pair is reported and the third is not |
| Maximal similarity | A long duplicate contains shorter duplicate windows | Only the maximal instance for the same locations is reported |
| Stable ordering | Discovery order and path separator representation vary | Normalized JSON findings and fingerprints are identical |
| Advisory mode | Findings exceed every configured limit | Report mode exits successfully and labels the findings advisory |
| Absolute check | A subject exceeds one configured limit without a baseline | Check mode fails and identifies the violated limit |
| Baseline regression | Existing over-limit subjects stay equal, improve, and worsen in separate runs | Equal and improved reports pass; the worsened report fails |
| Stale baseline | A baseline subject is deleted | The command reports the stale entry without failing solely for staleness |
| Invalid source | One selected module does not parse | The command fails and does not emit a clean metric summary |
| Invalid types | Selected source parses but has a type error | Metrics are reported without presenting the type error as a metric diagnostic |
| Baseline write safety | The target already exists | Baseline writing refuses to replace it |
| Truncated human output | Findings exceed `max_findings` | Policy uses the complete set; human output names the omitted count and the JSON evidence command |

CLI parsing, human output, JSON shape, exit status, and atomic baseline writes
must have integration coverage in `veln-cli`. Metric calculation and baseline
comparison must have table-driven unit coverage in a reusable metrics library.

## Bounded Analysis Requirement

Similarity analysis must not compare every source region with every other
source region. This internal constraint is normative because an unrestricted
pairwise scan would make the command unsuitable for generated or agent-grown
projects.

A generated benchmark must contain unrelated functions, repeated functions,
and repeated token prefixes at three adjacent sizes. It must report wall time,
user CPU time, peak resident memory, source token count, candidate count, and
reported instance count. Doubling unrelated input tokens must not increase
median user CPU time or peak resident memory by more than three times between
adjacent sizes on the same machine and build profile.

The benchmark is review evidence, not a portable CI time limit. Normal CI must
include a structural regression test that bounds candidate growth for the same
generated family.

## Implementation Guidance

This section is not normative except for the bounded-analysis constraint.

Create a reusable Veln metrics library rather than placing calculation in the
CLI command module. Reuse `veln-project` discovery and the lowered surface AST.
Preserve per-source module identity before merged project analysis discards
file grouping.

Use source `UseOrigin` to exclude implicit prelude edges. Compute graph values
from canonical module identities rather than from textual aliases. Derive
similarity tokens from the Veln lexer so formatting and comments do not become
semantic input.

Candidate indexing or fingerprinting can keep similarity analysis bounded.
The implementation must verify candidate regions against their complete
normalized token sequences before reporting equality; a fingerprint collision
must not create a finding.

The existing Rust-only `veln-code-metrics` remains a repository maintenance
tool. Do not make the public command depend on `syn` or accept Rust source.

## Non-Goals

- Do not change Veln syntax, typing, effect, contract, or runtime semantics.
- Do not make metric limits part of `veln check`.
- Do not claim that one threshold is universally healthy for every project.
- Do not automatically rewrite functions, split modules, or deduplicate code.
- Do not automatically update a baseline during `--check`.
- Do not measure dependencies inside external packages in the first slice.
- Do not include generated or doctest-derived sources in similarity analysis.
- Do not add approximate, identifier-insensitive, or semantic clone detection
  in the first slice.
- Do not use a single combined maintainability score that hides the underlying
  measurements.

## Planned Verification Commands

Implementation must make these repository-relative checks available:

```sh
bash scripts/agent-test -p veln-metrics
bash scripts/agent-test -p veln-cli --test toolchain_harness
bash scripts/agent-run cargo run --locked -p veln-cli -- metrics --json examples/specification/metrics
bash scripts/benchmark-veln-metrics compare SMALL MEDIUM LARGE
```

The metrics crate, examples, and benchmark command do not exist until this
proposal is implemented.

## Completion Boundary

This proposal is complete only when all acceptance cases pass, the generated
candidate-growth guard runs in normal CI, and the controlled benchmark meets
the bounded-analysis requirement.

Completion must add current command behavior to
`../specification/commands-full.md`, route it from
`../specification/commands.md`, specify JSON behavior through
`../specification/json-output.md`, and promote executable cases under
`../../examples/specification/metrics/`.

After completion, move this document to
`../reference/implemented-proposals/` and remove it from the proposal catalog.
