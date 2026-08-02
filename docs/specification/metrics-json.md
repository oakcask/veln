---
review-when: The metrics command JSON schema or executable metrics cases change.
---

# Metrics JSON

`veln metrics --json` and `veln metrics --check --json` emit schema version
`veln-metrics-json/v0`.

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
clean check report. Any `[tool.metrics]` field other than `deny_cycles`, or a
`deny_cycles` value other than `"true"` or `"false"`, is a manifest command
error at the field span.

A successful check keeps the complete metrics report and adds `check.mode:
"check"`, `check.enabled_policies`, `check.result: "pass"`, and an empty
`check.violations` array. A cycle violation exits non-zero with
`status: "policy_violation"` and keeps the complete report. Each violation
names `policy: "deny_cycles"`, the cycle members, a concrete closed path, and
guidance to review module ownership and dependency direction.

The JSON document contains:

- `tool.name`, `tool.version`, `command`, `status`, and `schema_version`;
- `project.root` and `project.selected_paths`, with normalized relative paths
  and no absolute paths;
- `modules`, sorted by descending `dependency_pressure`, descending
  `fan_out`, descending `fan_in`, then module identity;
- `edges`, sorted by source module and target module;
- `cycles`, with sorted members and at least one concrete closed edge `path`;
- `abc_subjects`, sorted by descending ABC magnitude, then project-relative
  path, declaration start offset, and subject kind;
- `summary`, with selected module, project module, internal edge, cycle,
  external dependency, ABC subject, and ABC contract-subject counts.

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

Executable evidence:

- The metrics `dependency-report` case checks advisory human output, graph
  counts, external dependency counts, and cycle paths.
- The metrics `path-selection` case checks JSON shape, selected subjects,
  containing graph counts, dependency edges, and cycle membership.
- The metrics `check-acyclic`, `check-cycle-human`, and `check-cycle-json`
  cases check enabled dependency-cycle policy success and violation output.
- The metrics `check-no-policy-human`, `check-no-policy-json`, and
  `check-invalid-policy-json` cases check configuration and manifest policy
  failures that do not return a clean check report.
- The metrics `abc-constructs` case checks counted ABC constructs, annotation
  and contract exclusion, and ABC summary fields.
- The metrics `abc-subject-kinds` case checks function and test subject kinds
  and excludes doctest-like documentation text from ABC subjects.
