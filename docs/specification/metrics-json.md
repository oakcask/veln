---
role: specification
authority: normative
update-when: The metrics command JSON schema or executable metrics cases change.
specification-coverage: usage=#metrics-json; behavior=#report-fields-and-ordering; limits=#partial-reports
---

# Metrics JSON

`veln metrics --json` and `veln metrics --check --json` emit schema version
`veln-metrics-json/v0`. `veln metrics --write-baseline PATH` writes baseline
schema version `veln-metrics-baseline/v0` with metric model
`veln-metrics-model/v0`.

Without `--check`, the command is report-only. A completed advisory analysis
returns `status: "ok"` and exits successfully even when dependency cycles or
large ABC values exist.
Discovery, manifest loading, parsing, module identity, import resolution, or
metric analysis errors fail without a clean metrics report. Type, effect, and
contract errors are not reported as metrics diagnostics and do not block this
syntax-and-module-graph report.

## Partial reports

When every source error is either `name.invalid_case` with `details.origin:
"source_path"` or an unresolved import caused by excluding such an identity,
the command emits a partial metrics report with `status: "incomplete"` and
exits non-zero. The report keeps the source-path casing diagnostics in
top-level `diagnostics`, emits `completeness.status: "partial"`, and lists
each excluded project-relative source path in `completeness.excluded_sources`
with `reason: "invalid_module_identity"`. The excluded sources do not create
module records, dependency edges, cycles, or graph-derived summary counts.
Parse-clean excluded sources remain eligible for selected path-based ABC and
whole-body similarity records. A parse error, unrelated unresolved import, or
any other non-qualifying source error keeps the ordinary diagnostic envelope
and emits no metrics report, completeness object, or check result.

## Policy results and baselines

With `--check`, `[tool.metrics] deny_cycles = "true"` enables dependency-cycle
policy enforcement. Omitted `deny_cycles` and `deny_cycles = "false"` leave no
enforceable policy enabled and fail as a command configuration error without a
clean check report. `[tool.metrics] similarity_min_tokens = "N"` sets the
minimum normalized token count for experimental whole-body similarity. It
defaults to `"60"` and must be a positive integer string. `[tool.metrics]
max_findings = "N"` sets the detailed human-output finding limit. It defaults
to `"50"`, applies independently to each detailed section, and must be a
positive integer string representable in the metrics JSON number domain. Any
other `[tool.metrics]` field, a `deny_cycles` value
other than `"true"` or `"false"`, an invalid `similarity_min_tokens` value, or
an invalid `max_findings` value is a manifest command error at the field span.

A successful check keeps the complete metrics report and adds `check.mode:
"check"`, `check.enabled_policies`, `check.result: "pass"`, and an empty
`check.violations` array. A cycle violation exits non-zero with
`status: "policy_violation"` and keeps the complete report. Each violation
names `policy: "deny_cycles"`, the cycle members, a concrete closed path, and
guidance to review module ownership and dependency direction.
With a partial report, a retained-graph cycle violation takes precedence and
uses `status: "policy_violation"` with `check.result: "fail"`. A partial check
without a retained-graph violation uses `status: "incomplete"`,
`check.result: "incomplete"`, an empty violation array, and a non-zero exit.

`--baseline PATH` is valid only with `--check`. With a baseline, `deny_cycles`
allows a current dependency cycle only when its members and cyclic edges are
subsets of one baseline cycle. Unchanged cycles and cycles that lost members
or cyclic edges pass. New cycles, new self-cycles, renamed-module cycles, and
cycles that added members or cyclic edges fail. A deleted baseline subject is
reported as stale but does not by itself fail the check. Unsupported baseline
schema or metric-model values are command errors.

When a check uses a baseline, the `check.baseline` object contains `path`,
`schema_version`, `metric_model`, and `stale_subjects`. The path is the
command-line baseline path normalized with `/` separators.
For partial baseline checks, a baseline module whose current source path is in
`completeness.excluded_sources` is omitted from `stale_subjects` and appears
in `completeness.excluded_baseline_subjects`, sorted by module identity.

`--write-baseline PATH` writes the complete current report fields described
below, replacing only the top-level `schema_version` with
`veln-metrics-baseline/v0` and adding top-level `metric_model` with value
`veln-metrics-model/v0`. It writes project-relative paths and does not write
absolute paths or source text. It refuses to overwrite an existing path and
refuses to write an incomplete partial report.

## Report fields and ordering

The JSON document contains:

- `tool.name`, `tool.version`, `command`, `status`, and `schema_version`;
- `diagnostics`, empty for complete reports and containing retained
  source-path casing diagnostics for partial reports;
- `project.root` and `project.selected_paths`, with normalized relative paths
  and no absolute paths. Metrics-owned project-relative paths use `/`
  separators in JSON, baseline JSON, and human locations;
- `completeness` only for partial reports, with `status`, excluded source
  paths, and excluded baseline subjects when a baseline check has them;
- `modules`, sorted by descending `dependency_pressure`, descending
  `fan_out`, descending `fan_in`, then module identity;
- `edges`, sorted by source module and target module, with canonical edge
  spans;
- `cycles`, sorted by first member and member count, with sorted members and
  at least one concrete closed edge `path`;
- `abc_subjects`, sorted by descending ABC magnitude, then project-relative
  path, declaration start offset, and subject kind;
- `similarities`, sorted by descending token count, then primary declaration
  path, primary declaration start offset, primary declaration kind, and
  fingerprint. Declarations inside each instance are sorted by declaration
  path, declaration start offset, and declaration kind;
- `summary`, with selected module, project module, internal edge, cycle,
  external dependency, ABC subject, ABC contract-subject, similarity
  fingerprint, similarity instance, and similarity region counts;
- `human_output`, with `max_findings`, `total_findings`, `omitted_findings`,
  and `truncated` for the corresponding human projection.

`human_output.total_findings` counts detailed human-output findings across
policy violations for checked reports, cycles, module rows, ABC subjects, and
whole-body similarity instances. The human projection includes the first
`human_output.max_findings` values from each category in that category's
canonical order. `human_output.omitted_findings` is the sum of values omitted
after applying that limit to each category independently. Each truncated human
section states its displayed, total, and omitted counts. Equivalent source
discovery orders and equivalent `/` or `\` project-relative path spellings
produce the same ordered report. The report arrays remain complete when
`human_output.truncated` is `true`. Baseline output does not include
`human_output`, and baseline content is independent of the human-output limit.

## Dependency graph

Each module record includes `module`, `path`, `generated`, `fan_in`,
`fan_out`, `dependency_pressure`, `external_dependency_count`, and `span`.
`dependency_pressure` is `fan_in * fan_out`.

The dependency graph contains project-owned modules. Source-written internal
`use` declarations create directed edges and duplicate imports between the same
pair of modules create one edge. Implicit standard-prelude imports do not
create edges. Dependency packages and embedded standard-library modules are
not metric subjects. A source-written import from a project-owned module to a
non-standard dependency package increments that module's
`external_dependency_count` without creating an internal edge.

Path arguments select the project-owned modules reported as module subjects.
The command still analyzes the containing project graph so selected modules
retain fan-in, fan-out, dependency pressure, and cycle membership calculated
from the complete project-owned graph. Dependency edges are reported when the
source or target module is selected.

## ABC subjects

Each ABC subject record describes one selected project-owned source function
or test declaration. It includes `identity`, `path`, `name`, `kind`,
`generated`, `contracts_included`, `abc`, and `span`. `kind` is `function` or
`test`. `identity` is the project-relative path followed by the declaration
name. The `abc` object contains integer `assignments`, `branches`, and
`conditionals` components plus the unrounded `magnitude` value as a decimal
string. Human output rounds that magnitude to one decimal place and labels the
measurement `ABC size`.

ABC counts only function and test bodies. Each `let` body line increments
`assignments`. Each call, effect performance, handler application, schema
decode expression, and schema encode expression increments `branches`. Each
`if` condition, `else if` condition, `match` expression, match arm,
short-circuit `and` or `or`, and `?` expression increments `conditionals`.
Nested expressions contribute to the containing declaration. Declaration
signatures, result bindings, type and effect annotations, and contract text do
not contribute. `contracts_included` is `false` for every ABC subject.

## Similarity records

Each similarity record describes one experimental exact whole-body similarity
instance among selected project-owned source function or test declarations. It
includes `identity`, `fingerprint`, `token_count`, `experimental`, and
`declarations`. `experimental` is `true`. `identity` is
`similarity:` followed by the normalized-token fingerprint. Each declaration
record includes `identity`, `path`, `name`, `kind`, `generated`, `span`, and
`body_span`. `kind` is `function` or `test`.

Similarity compares complete declaration bodies after removing comments,
documentation text, whitespace, and formatting-only newlines. Identifier
spelling and literal token text remain significant. A similarity instance is
reported only when two or more declarations have equal complete normalized
body token sequences and the sequence contains at least the effective
`similarity_min_tokens` count. Partial-body matches are not reported.
Generated and doctest-derived declarations are excluded. Similarity is
advisory: it never creates a `--check` policy violation, and baseline checks do
not fail when a duplicate pair changes together.

Similarity analysis creates one declaration fingerprint for each selected
eligible declaration whose normalized body has at least the effective
`similarity_min_tokens` count. Each eligible declaration can appear in at most
one similarity instance. For `N` such declarations, the report has at most `N`
similarity regions and at most `floor(N / 2)` similarity instances. Similarity
is advisory and has no portable timing limit; the selection and normalization
rules above define the public result.

## References

The metrics report is produced by `crates/veln-cli/src/commands/metrics.rs`; CLI metrics assertions verify field shapes, ordering, policy results, partial reports, and baselines.
