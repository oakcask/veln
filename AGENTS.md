* Think in English.
* Do not write full path in files.
* Do not write calendar dates in documentation, source, comments, filenames,
  or metadata unless testing date-pattern behavior or preserving an externally
  defined identifier such as a URL.
* Treat `docs/specification/` as the source of current implemented
  behavior. Treat `docs/proposals/` as planned or accepted work only; do not
  cite or edit proposal text as current behavior unless the matching
  specification page also states it.
* When implementing or completing proposal work, use
  `$proposal-implementation-audit` to promote implemented behavior into
  `docs/specification/` and `examples/specification/`, and to remove completed
  work from `docs/proposals/`.
* When adding, moving, or reclassifying documentation, use
  `$docs-progressive-disclosure` to keep documentation routes short and status
  boundaries consistent.
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
