---
role: specification
authority: normative
update-when: The veln metrics command output, policy check, baseline behavior, source selection, or partial analysis contract changes.
specification-coverage: usage=#metrics-command; behavior=#selection-and-partial-analysis; limits=#limits
---

# Metrics Command

Use `veln metrics [PATH ...]` for advisory dependency, ABC-size, and
experimental exact whole-body similarity measurements. It shares package-root
selection and parse-clean module loading with the other analysis commands.
Without `--check`, cycles, large ABC values, and duplicate bodies are
reported but do not make a completed analysis fail.

## Selection and partial analysis

Paths select project-owned module subjects while the complete project graph is
analyzed for fan-in, fan-out, dependency pressure, and cycles. A partial report
is produced only when every source error is a source-path-derived
`name.invalid_case` or an unresolved import caused by excluding that identity.
The report retains those diagnostics and excluded project-relative paths, omits
excluded identities from graph records and graph summaries, and exits
unsuccessfully. Parse-clean excluded sources remain eligible for explicitly
selected ABC and similarity subjects. Other source errors produce the ordinary
diagnostic envelope and no metrics report.

## Policies and baselines

`--json` emits [metrics-json.md](metrics-json.md). `--check` enables the
manifest policy `[tool.metrics] deny_cycles = "true"`; omitted or false
leaves no policy enabled and is a command error. `max_findings` is a positive
integer string, defaults to `50`, and limits each human detail section
independently. `similarity_min_tokens` is a positive integer string and
defaults to `60`.

Use `--baseline PATH` only with `--check`. A current cycle passes when its
members and cyclic edges are subsets of one baseline cycle. New cycles,
self-cycles, renamed modules, or added members/edges fail. A deleted baseline
subject is stale but does not itself fail; a subject excluded by partial
analysis is reported separately.

Use `--write-baseline PATH` to write a complete baseline. It uses
`veln-metrics-baseline/v0` and `veln-metrics-model/v0`, contains normalized
project-relative paths, excludes source text and absolute paths, refuses an
existing target, and refuses incomplete reports. It conflicts with
`--check` and `--json`.

## Limits

Similarity is advisory and never creates a policy violation. Human output may
be truncated by `max_findings`, but JSON arrays, summaries, policy
evaluation, and baseline content use the complete finding set.

## References

Implementation: `crates/veln-cli/src/commands/metrics.rs`. Metrics harness
coverage verifies reports, policies, partial graphs, baselines, and limits.
