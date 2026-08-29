---
role: specification
authority: normative
update-when: The veln metrics command output, policy check, baseline behavior, source selection, or partial analysis contract changes.
---

# Metrics Command

`veln metrics` reports advisory module dependency metrics, ABC size metrics,
and experimental exact whole-body similarity for project-owned Veln source. It
follows the shared command analysis route for source discovery and parse-clean
module loading. Without `--check`, the command exits successfully when
analysis completes, even if dependency cycles, large ABC values, or duplicate
whole bodies are present.

When every source error is either a source-path-derived `name.invalid_case`
diagnostic or an unresolved import caused by excluding such an identity,
metrics returns a partial report instead of the ordinary source diagnostic
envelope. Exclusion-caused unresolved imports are limited to imports to an
excluded source's source-kind-aware visible identity and imports from an
excluded source to another project-owned visible identity. The command keeps
the source-path casing diagnostics, excludes the invalid source identities
from module records, dependency edges, cycles, and graph-derived summary
counts, and reports the excluded project-relative paths. ABC and whole-body
similarity still include parse-clean invalid sources when their paths are
selected. Advisory partial reports exit non-zero.

`--json` emits the metrics JSON report specified in [metrics-json.md](metrics-json.md).
`--check` applies enabled metrics policy from `[tool.metrics]`. The current
enforceable policy is `deny_cycles = "true"`. A check with no enabled policy
is a command error.

`--write-baseline PATH` writes the current complete metrics report as
baseline schema `veln-metrics-baseline/v0` with metric model
`veln-metrics-model/v0`. The baseline records project-relative paths and does
not record absolute paths or source text. The command writes through a
temporary file in the target directory and refuses to overwrite an existing
target. It refuses partial reports without creating or replacing the requested
path. `--write-baseline` conflicts with `--check` and `--json`.

`--baseline PATH` is valid only with `--check`. The baseline is loaded
explicitly from the command line; the command does not load a manifest
baseline implicitly. Unsupported baseline schema or metric model values are
comparison errors. A baseline subject that no longer exists in the current
report is reported as stale but does not by itself fail the check. A baseline
subject whose current source identity is excluded by partial analysis is
reported as an excluded baseline subject instead of stale.

Human report output begins with summary counts, then cycles, then module rows,
then ABC size, and then `Whole-body similarity (experimental)`. Each
similarity instance names one primary declaration with its declaration and body
source locations. The remaining declarations are related locations.
Similarity output does not instruct maintainers to deduplicate code
mechanically, and similarity never creates a policy violation under `--check`.

`[tool.metrics] max_findings = "N"` limits each detailed human-output section
to its first `N` findings in canonical order. The policy-violation, cycle,
module-row, ABC-subject, and whole-body-similarity sections apply the limit
independently, so findings in one section do not hide every finding in a later
section. The default is `50`. The value must be a positive integer string
representable in the metrics JSON number domain. Zero, malformed strings,
values outside the implementation's integer range, and values outside that
JSON number domain are manifest errors at the value span. A truncated section
states its displayed, total, and omitted counts and identifies `veln metrics
--json` as the complete evidence route. The final summary states the exact
omitted count across all sections. Summary counts, section headings, policy
status, baseline status, related lines for a displayed finding, JSON arrays,
policy evaluation, and baseline content use the complete finding set.

When `deny_cycles = "true"` is checked with a baseline, a current cycle is
allowed only when its member set and cyclic edge set are subsets of one
baseline cycle. This allows unchanged cycles and cycles that lost members or
cyclic edges. New cycles, self-cycles without a matching baseline allowance,
renamed-module cycles, and cycles with added members or cyclic edges fail.
For partial analysis, a known retained-graph cycle still fails with a policy
violation. Without a known retained-graph violation, the check result is
incomplete and exits non-zero.
