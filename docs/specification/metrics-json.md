---
role: specification
authority: normative
update-when: The metrics command JSON schema or executable metrics cases change.
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

`--write-baseline PATH` writes the complete current report fields described
below, replacing only the top-level `schema_version` with
`veln-metrics-baseline/v0` and adding top-level `metric_model` with value
`veln-metrics-model/v0`. It writes project-relative paths and does not write
absolute paths or source text. It refuses to overwrite an existing path.

The JSON document contains:

- `tool.name`, `tool.version`, `command`, `status`, and `schema_version`;
- `project.root` and `project.selected_paths`, with normalized relative paths
  and no absolute paths. Metrics-owned project-relative paths use `/`
  separators in JSON, baseline JSON, and human locations;
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
similarity regions and at most `floor(N / 2)` similarity instances. The
explicit `scripts/benchmark-metrics-similarity` review command uses generated
unrelated functions, repeated functions, and repeated token-prefix functions at
adjacent sizes. It reports source token count, declaration fingerprint count,
similarity instance count, reported region count, wall time, user CPU time,
peak resident memory, medians, and adjacent-size ratios. The benchmark checks
that median user CPU time and median peak resident memory do not grow by more
than three times between adjacent sizes on the same machine and build profile;
this timing check is review evidence, not a portable CI limit.

Executable evidence:

- The metrics `dependency-report` and `dependency-report-json` cases check
  advisory human and JSON output, graph counts, external dependency counts,
  and cycle paths.
- The metrics `path-selection` case checks JSON shape, selected subjects,
  containing graph counts, dependency edges, and cycle membership.
- The metrics `check-acyclic`, `check-cycle-human`, and `check-cycle-json`
  cases check enabled dependency-cycle policy success and violation output.
- The metrics `check-no-policy-human`, `check-no-policy-json`,
  `check-invalid-policy-json`, and `check-unsupported-policy-json` cases check
  configuration and manifest policy failures that do not return a clean check
  report.
- The metrics `human-output-truncated`,
  `human-output-truncated-json`, `check-human-output-truncated`,
  `check-human-output-truncated-json`, `invalid-max-findings-human`, and
  `invalid-max-findings-json` cases check the shared human-output budget,
  exact omitted counts, unchanged JSON evidence, failing checked status under
  truncation, and invalid `max_findings` diagnostics. Their human-output
  fixture fragments preserve report-section order even when inline assertions
  and file-backed assertions are split in the case manifest.
- The metrics `baseline-write` and `baseline-existing-file` cases check
  baseline generation, baseline file shape, and overwrite refusal.
- The metrics `baseline-check-pass-json`,
  `baseline-check-regression-json`, `baseline-stale-human`, and
  `baseline-unsupported-schema-json`, and
  `baseline-unsupported-metric-model-json` cases check baseline-aware cycle
  allowances, regressions, stale subject reporting, and unsupported version
  comparison errors.
- The `metrics_baseline_check_preserves_report_fields` CLI integration test
  checks that a baseline check preserves the advisory ABC subjects, graph
  measurements, ordering, and ordinary report fields from the matching
  no-baseline JSON report.
- The metrics `abc-constructs` case checks counted ABC constructs, annotation
  and contract exclusion, and ABC summary fields.
- The metrics `abc-subject-kinds` case checks function and test subject kinds
  and excludes doctest-like documentation text from ABC subjects.
- The metrics `similarity-formatted-equal`, `similarity-partial-body`,
  `similarity-human-output`, and `check-similarity-baseline-advisory` cases
  check exact whole-body similarity, identifier-sensitive exclusion,
  partial-body exclusion, human output placement and locations, summary
  counts, and advisory baseline behavior under `--check`.
- The metrics `stable-ordering` and `stable-ordering-human` cases check
  public CLI ordering for selected paths, modules, edges, cycles, ABC
  subjects, same-token-count similarity instances, similarity declarations,
  and the corresponding human prefix, with file-backed and inline expected
  fragments kept in manifest order. The
  `metrics_cli_output_is_stable_for_reversed_input_order` CLI integration test
  checks byte-for-byte stable JSON and human output for reversed input order
  when the detailed finding set is truncated.
- The `canonical_path_ordering_survives_source_insertion_order_and_separators`
  metrics crate test checks graph, ABC, similarity, baseline JSON, human
  locations, and path-bearing identities across reversed source insertion
  order and equivalent `/` or `\` project-relative path spellings. The
  `renders_similarity_fingerprint_tiebreak_order_in_public_outputs` metrics
  crate test checks the final fingerprint tie-break in JSON and human output.
- The `generated_similarity_workload_preserves_pipeline_bounds` metrics crate
  test checks the parsed source and report pipeline with unrelated bodies, one
  large equivalence class, many two-declaration equivalence classes, and
  repeated token prefixes.
