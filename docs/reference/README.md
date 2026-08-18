---
role: routing
update-when: A reference document is added, moved, reclassified, or removed.
---

# Reference

Stable rationale and source-support material lives here. Use
`../specification/` for current language behavior, then return here only when a
behavior page needs rationale, source support, or toolchain reference material.

## Read First

- Current language behavior: [specification/README.md](../specification/README.md).
- Task-specific behavior route: [specification/topic-map.md](../specification/topic-map.md).
- Rationale route: [specification/source-decisions.md](../specification/source-decisions.md).

## Fast Routes

- Implemented behavior: [specification/topic-map.md](../specification/topic-map.md).
- Legacy grammar route: [grammar.md](grammar.md).
- Human diagnostics:
  [specification/diagnostics-json.md](../specification/diagnostics-json.md).
- Machine-readable command output:
  [specification/json-output.md](../specification/json-output.md).
- CLI integration test harness:
  [toolchain-test-harness.md](toolchain-test-harness.md).
- HTTP/2 public symbol migration and residual-name classification:
  [http2-standard-module-migration.md](http2-standard-module-migration.md).
- Implemented proposal records:
  [implemented-proposals/README.md](implemented-proposals/README.md).
- Implemented rationale: [source-decisions/README.md](source-decisions/README.md),
  then [source-decisions/topic-map.md](source-decisions/topic-map.md) when the
  category is unclear.
- Source support behind rationale: [bibliography/README.md](bibliography/README.md).

## Read When

- Use `../specification/` before changing implemented behavior, tests,
  diagnostics, commands, JSON output, runtime behavior, or examples.
- Use [toolchain-test-harness.md](toolchain-test-harness.md) before changing
  CLI integration case layout or assertion policy.
- Use `implemented-proposals/` only for completed proposal history or
  completion evidence after checking current behavior.
- Use `source-decisions/` after a language page needs rationale; start with
  its README before opening category pages or records.
- Use `bibliography/` after a rationale or claim needs source support.

## Route Boundaries

- Planned behavior belongs in `../proposals/`.
- Historical gap evidence belongs in the matching proposal or reference page.
- Implemented language behavior belongs in
  [specification/README.md](../specification/README.md).
- Check the source and destination README files before moving text between
  route areas.

## Skip Unless Needed

- Do not open `*-full.md` files before a short route page names the needed
  section.
- Do not scan individual source-decision records before a topic route points to
  one.
- Do not use proposal text as current behavior when `../specification/` has a
  matching page.
