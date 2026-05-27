# Reference

Stable reference material lives here. Use it for implemented behavior and the
rationale or sources needed to maintain that behavior.

## Read First

- Current language behavior: [language/README.md](language/README.md).
- Task-specific behavior route: [language/topic-map.md](language/topic-map.md).
- Rationale route: [language/source-decisions.md](language/source-decisions.md).

## Fast Routes

- Implemented behavior: [language/topic-map.md](language/topic-map.md).
- Legacy grammar route: [grammar.md](grammar.md).
- Human diagnostics: [language/diagnostics-json.md](language/diagnostics-json.md).
- Machine-readable command output:
  [language/json-output.md](language/json-output.md).
- CLI integration test harness:
  [toolchain-test-harness.md](toolchain-test-harness.md).
- Implemented rationale: [source-decisions/README.md](source-decisions/README.md),
  then [source-decisions/topic-map.md](source-decisions/topic-map.md) when the
  category is unclear.
- Source support behind rationale: [bibliography/README.md](bibliography/README.md).

## Read When

- Use `language/` before changing implemented behavior, tests, diagnostics,
  commands, JSON output, runtime behavior, or examples.
- Use [toolchain-test-harness.md](toolchain-test-harness.md) before changing
  CLI integration case layout or assertion policy.
- Use `source-decisions/` after a language page needs rationale; start with
  its README before opening category pages or records.
- Use `bibliography/` after a rationale or claim needs source support.

## Route Boundaries

- Planned behavior belongs in `../proposals/`.
- Gap evidence belongs in `../reviews/`.
- Implementation order belongs in `../phases/`.
- Use [../document-status.md](../document-status.md) before moving text between
  route areas.

## Skip Unless Needed

- Do not open `*-full.md` files before a short route page names the needed
  section.
- Do not scan individual source-decision records before a topic route points to
  one.
- Do not use proposal, review, or phase text as current behavior when
  `language/` has a matching page.
