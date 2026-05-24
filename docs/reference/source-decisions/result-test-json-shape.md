# Discussion Result: Test JSON Shape

Status: implemented

## Picked Question

- What first JSON shape should `veln test --json` use for suite status, test
  cases, blocked runs, runtime failures, diagnostics, and captured events?

## Decision

Use one run-level JSON object with a small stable envelope, deterministic
summary counts, top-level gate diagnostics, suite-level errors, and per-case
records. Keep `veln test --json` native JSON, not JUnit XML, TAP text, or CTRF,
while preserving enough structure for later exporters.

The first shape is:

```json
{
  "schema_version": "veln-test-json/v0",
  "command": "test",
  "status": "failed",
  "selection": {
    "mode": "explicit",
    "targets": ["tests/order_test.veln"],
    "confidence": "complete",
    "reason": "user_selected"
  },
  "summary": {
    "total": 2,
    "passed": 1,
    "failed": 1,
    "skipped": 0,
    "todo": 0,
    "blocked": 0,
    "errors": 0
  },
  "diagnostics": [],
  "suite_errors": [],
  "cases": [
    {
      "id": "case-1",
      "name": "summarizes valid order",
      "kind": "test",
      "status": "passed",
      "source": {
        "file": "tests/order_test.veln",
        "node_id": "test-3",
        "span": {
          "start": {"line": 3, "column": 1},
          "end": {"line": 8, "column": 4}
        }
      },
      "reason": null,
      "failure": null,
      "events": [],
      "diagnostics": []
    }
  ]
}
```

Required run fields are `schema_version`, `command`, `status`, `selection`,
`summary`, `diagnostics`, `suite_errors`, and `cases`.

Required case fields are `id`, `name`, `kind`, `status`, `source`, `reason`,
`failure`, `events`, and `diagnostics`.

## Status Model

Run `status` starts with `passed`, `failed`, `blocked`, or `error`.

- `passed`: all selected cases passed or were expected non-passing cases.
- `failed`: at least one selected case failed.
- `blocked`: static gates, holes, discovery, or reachability checks prevented
  selected cases from running.
- `error`: the test runner failed outside user-code semantics.

Case `status` starts with `passed`, `failed`, `skipped`, `todo`, `blocked`, or
`error`.

`todo` means an expected-failing executable TODO case. A TODO case that passes
should keep `status: "passed"` and set `reason: "todo_now_passing"` so the
human output can ask for promotion without making the whole run fail.

## Failure And Error Records

Case `failure` is `null` for `passed`, `skipped`, and ordinary `todo` cases.
For failures, use a small typed record:

```json
{
  "kind": "assertion",
  "message": "expected total cents to match",
  "expected": "1250",
  "actual": "1200",
  "span": {
    "file": "tests/order_test.veln",
    "start": {"line": 7, "column": 3},
    "end": {"line": 7, "column": 34}
  },
  "details": {
    "assertion": "equal"
  }
}
```

`failure.kind` starts with `assertion`, `contract`, `panic`, `output`, or
`runtime`. A runtime contract failure inside a case uses `kind: "contract"` and
embeds the structured contract error fields decided by
[Runtime Contract Failure Reporting](../../proposals/agent-language-spec-wall/result-runtime-contract-failure-reporting.md).
Discovery failures, static-gate failures, or runtime failures outside a selected
case belong in top-level `diagnostics` or `suite_errors`.

## First-Slice Rules

- `veln test --json` should run the same static gates as `veln run` before
  executing selected cases. Gate diagnostics use the existing `check --json`
  diagnostic envelope in the top-level `diagnostics` array.
- If gate diagnostics block execution, use `status: "blocked"`, keep `cases`
  empty unless cases were already discovered, and fill `summary.blocked`.
- `selection.targets` uses source-relative paths or stable target names, never
  machine-local absolute paths.
- `selection.confidence` starts with `complete`, `partial`, or `unknown`, reusing
  the conservative affected-test-selection rule.
- Captured stdio output appears in each case's `events` array using the event
  shape from [Stdio API and Output Events](result-stdio-api-and-output-events.md).
- `summary` counts are derived from `cases` plus suite-level `errors`; they are
  included so agents and CI do not have to recompute the common outcome.
- The first JSON shape excludes wall-clock timestamps, process IDs, and host
  paths from required fields. Optional timing metrics can be added later under a
  clearly named `metrics` object without changing result identity.
- Golden tests should assert required keys, enum values, and source-relative
  identity, not prose in `message`.

## Rationale

The existing diagnostic decision already gives Veln a stable-envelope pattern:
common routing fields first, kind-specific evidence beneath them. `test --json`
should use the same design at the run and case levels. Agents need to decide
whether a run passed, failed, was blocked by static gates, or failed as a tool
error before they inspect detailed evidence.

TAP is useful because it has a simple plan, per-test status, TODO/SKIP
directives, bail-out behavior, and optional structured diagnostic data next to
test points. Veln should keep those concepts, but JSON should make the plan,
summary, and diagnostics explicit rather than requiring line parsing.

JUnit Platform Reporting and Open Test Reporting show why Veln should not make
legacy JUnit XML its primary output. JUnit keeps reporting behind listeners and
distinguishes event-based reporting from legacy XML. Veln should do the same:
native JSON for first-slice agents, possible exporters later for CI systems that
prefer XML.

CTRF is the closest JSON precedent: it puts a tool, summary counts, per-test
status, and optional extensions in one report. Veln should copy the compact
shape, not the exact schema, because Veln also needs source spans, static gate
diagnostics, hole/blocking information, and captured stdio events.

Regression-test-selection research supports reporting selection confidence.
Rothermel and Harrold define safety in terms of not excluding fault-revealing
tests under the technique's assumptions. The earlier Veln decision already says
selection must widen when evidence is incomplete; the JSON result should expose
that confidence so a caller can distinguish a complete explicit run from a
best-effort affected run.

Compiler-diagnostic and question-centered debugging research supports placing
repair evidence next to the failing case. Assertion expected/actual values,
source spans, contract blame, and captured output events answer the practical
questions an agent asks after a failure: what failed, where did it fail, what
was expected, what was observed, and which static or runtime evidence constrains
the repair?

## Open Details

Test declaration syntax is now defined by
[Test Declaration Syntax](../../proposals/agent-language-spec-wall/result-test-declaration-syntax.md). This result does
not define parameterized tests, parallel execution ordering, flaky-test retry
records, timing metrics, coverage, JUnit/TAP/CTRF exporters, or a stable schema
beyond `veln-test-json/v0`.

The exact `failure.details` payloads for assertion and output mismatches can be
refined when assertion syntax and expected-output assertions are implemented.

## Consequence

The first implementation can now produce useful machine-readable `test` output
without waiting for a complete reporting ecosystem. Agents get a deterministic
run summary, blocked-run evidence, per-case failures, static diagnostics, and
captured events in one result object, while future exporters remain additive.

## References

- Armstrong, A., Lester, A., & Schwern, M. G. (2007). *TAP13 - The Test
  Anything Protocol v13*. Test Anything Protocol.
  https://testanything.org/tap-version-13-specification.html
- JUnit Team. (2026). *JUnit Platform Reporting*. JUnit User Guide 6.0.3.
  https://docs.junit.org/6.0.3/advanced-topics/junit-platform-reporting.html
- CTRF contributors. (2026). *CTRF JSON Schema*. Common Test Report Format.
  https://ctrf.io/docs/full-schema
- Rothermel, G., & Harrold, M. J. (1997). A safe, efficient regression test
  selection technique. *ACM Transactions on Software Engineering and
  Methodology*, 6(2), 173-210. https://doi.org/10.1145/248233.248262
- Rothermel, G., & Harrold, M. J. (1998). Empirical studies of a safe
  regression test selection technique. *IEEE Transactions on Software
  Engineering*, 24(6), 401-419. https://doi.org/10.1109/32.689399
- Barik, T., Ford, D., Murphy-Hill, E., & Parnin, C. (2018). How Should
  Compilers Explain Problems to Developers? *ESEC/FSE 2018*.
  https://doi.org/10.1145/3236024.3236040
- Ko, A. J., & Myers, B. A. (2004). Designing the whyline: A debugging
  interface for asking questions about program behavior. *CHI 2004*, 151-158.
  https://doi.org/10.1145/985692.985712
