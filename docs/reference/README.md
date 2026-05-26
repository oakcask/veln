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
- Human diagnostics or machine-readable command output:
  [language/json-output.md](language/json-output.md).
- Implemented rationale: [source-decisions/topic-map.md](source-decisions/topic-map.md).
- Source support behind rationale: [bibliography/README.md](bibliography/README.md).

## Read When

- Use `language/` before changing implemented behavior, tests, diagnostics,
  commands, JSON output, runtime behavior, or examples.
- Use `source-decisions/` after a language page needs rationale.
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
