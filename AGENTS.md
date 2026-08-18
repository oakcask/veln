* Think in English.
* Do not write full path in files.
* Name repository-maintenance Cargo packages `veln-repo-*` and place them
  under `tools/`. Reserve other `veln-*` package names for toolchain components.
* Do not write calendar dates in documentation, source, comments, filenames,
  or metadata unless testing date-pattern behavior or preserving an externally
  defined identifier such as a URL.
* Treat `docs/specification/` as the source of current implemented
  behavior. Keep only `role: proposal` proposal pages in `docs/proposals/`;
  remove or relocate rejected, superseded, implemented, or otherwise closed
  proposals. Do not cite or edit proposal text as current behavior unless the
  matching specification page also states it.
* Select proposal targets only from the Ready section of
  `docs/proposals/README.md`. If no ready implementation target exists, report
  that there is no target.
* When implementing or completing proposal work, use
  `$proposal-implementation-audit` to promote implemented behavior into
  `docs/specification/` and `examples/specification/`, and to remove completed
  work from `docs/proposals/`.
* When adding, moving, or reclassifying documentation, use
  `$docs-progressive-disclosure` to keep documentation routes short and role,
  authority, and lifecycle boundaries consistent.
* Every added or changed Markdown document under `docs/` must declare its
  `role:` and one concrete `update-when:` trigger in YAML frontmatter. Declare
  `authority:` and exceptional `status:` only where the role permits them. Use
  `$docs-progressive-disclosure` to classify and validate the metadata.
* When creating or substantially revising any document that specifies
  behavior, including proposals, design notes, and reference material, use
  `$verifiable-specification-writing`.
* When adding or changing human diagnostics, keep the primary message focused
  on the specific failed fact at the reported span. Put causes, provenance,
  repair hints, and other related locations in `related` notes, and add human
  output coverage when related context is expected.
* When adding or changing CI-visible messages, make the required action and why
  it matters clear; use `$ci-message-policy`.
* When investigating slow tests or changing analysis code that processes large
  generated inputs, use `$performance-regression-audit` before reporting the
  work complete.
* When running broad tests, stress cases, generated-input tests, or analysis
  commands that may process large inputs, use `$agent-safe-local-runs`.
* Do not split Rust source into numbered bucket file series such as
  `parser01.rs` / `parser02.rs` or `part01.rs` / `part02.rs`. Module and file
  names must describe the responsibility or concept they own, especially when
  responding to code-metrics or complexity logs.
