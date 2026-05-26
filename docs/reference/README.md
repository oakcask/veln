# Reference

Stable reference material lives here. Use this directory for behavior
implemented in the current workspace; leave planned work in `../proposals/`
until code and tests support it.

## Read First

- [language/README.md](language/README.md): current language behavior.
- [language/topic-map.md](language/topic-map.md): task-oriented route to the
  smallest language reference page.
- [language/source-decisions.md](language/source-decisions.md): route to
  rationale when current behavior needs context.

## Fast Routes

- Implemented behavior: `language/README.md`, then `language/topic-map.md`.
- Rationale for implemented behavior: `language/source-decisions.md`, then one
  source-decision category.
- Research support behind rationale: `bibliography/README.md`, then one short
  bibliography route page.

## Read When

- Source language, contract, hole, command, JSON-output, runtime, or example
  changes: use [language/topic-map.md](language/topic-map.md).
- Implemented rationale: start with
  [language/source-decisions.md](language/source-decisions.md), then use
  [source-decisions/topic-map.md](source-decisions/topic-map.md).
- Research-source routes behind source decisions:
  [bibliography/README.md](bibliography/README.md).

## Route Boundaries

- Use `language/` for behavior users can rely on in the current workspace.
- Use `source-decisions/` only when implemented behavior needs rationale.
- Use `bibliography/` only when auditing research support behind the rationale.
- Use proposal, phase, or review directories when behavior is planned, disputed,
  or being verified rather than already implemented.

## Skip Unless Needed

- Do not open `*-full.md` files before a short route page names the needed
  section.
- Do not scan individual source-decision records before a topic route points to
  one.
- Do not open bibliography full records before a source family, claim, or
  reference route selects the relevant source.
- Use [../document-status.md](../document-status.md) before moving text between
  proposal, review, phase, and reference areas.
