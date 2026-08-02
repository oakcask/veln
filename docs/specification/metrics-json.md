---
review-when: The metrics command JSON schema or executable metrics cases change.
---

# Metrics JSON

`veln metrics --json` emits schema version `veln-metrics-json/v0`.

The command is report-only in this slice. A completed analysis returns
`status: "ok"` and exits successfully even when dependency cycles exist.
Discovery, manifest loading, parsing, module identity, import resolution, or
metric analysis errors fail without a clean metrics report. Type, effect, and
contract errors are not reported as metrics diagnostics and do not block this
syntax-and-module-graph report.

The JSON document contains:

- `tool.name`, `tool.version`, `command`, `status`, and `schema_version`;
- `project.root` and `project.selected_paths`, with normalized relative paths
  and no absolute paths;
- `modules`, sorted by descending `dependency_pressure`, descending
  `fan_out`, descending `fan_in`, then module identity;
- `edges`, sorted by source module and target module;
- `cycles`, with sorted members and at least one concrete closed edge `path`;
- `summary`, with selected module, project module, internal edge, cycle, and
  external dependency counts.

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

Executable evidence:

- The metrics `dependency-report` case checks advisory human output, graph
  counts, external dependency counts, and cycle paths.
- The metrics `path-selection` case checks JSON shape, selected subjects,
  containing graph counts, dependency edges, and cycle membership.
