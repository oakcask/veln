* Do not write machine-specific absolute filesystem paths in repository files.
* Name repository-maintenance Cargo packages `veln-repo-*` and place them
  under `tools/`. Reserve other `veln-*` package names for toolchain components.
* Do not write calendar dates in durable documentation, source, comments, or
  filenames unless testing date-pattern behavior or preserving an externally
  defined identifier such as a URL. Machine-maintained cache metadata may use
  dates when its schema or freshness logic requires them.
* Treat `docs/specification/` as the source of current implemented
  behavior. Keep only `role: proposal` proposal pages in `docs/proposals/`;
  remove or relocate rejected, superseded, implemented, or otherwise closed
  proposals. Do not cite or edit proposal text as current behavior unless the
  matching specification page also states it.
* When selecting, implementing, completing, reviewing, or cleaning up proposal
  work, use `$proposal-implementation-audit`.
* When adding, moving, classifying, or reorganizing documentation, or changing
  documentation routes or metadata, use `$docs-progressive-disclosure`.
* When creating or substantially revising any document that specifies
  behavior, including proposals, design notes, and reference material, use
  `$verifiable-specification-writing`.
* When adding or changing human diagnostics, keep the primary message focused
  on the specific failed fact at the reported span. Put causes, provenance,
  repair hints, and other related locations in `related` notes, and add human
  output coverage when related context is expected.
* When adding or changing CI-visible messages, make the required action and why
  it matters clear; use `$ci-message-policy`.
* When adding, changing, or reviewing GitHub Actions workflow design, use
  `$github-actions-design`.
* When investigating slow tests or changing analysis code that processes large
  generated inputs, use `$performance-regression-audit` before reporting the
  work complete.
* When running broad tests, stress cases, generated-input tests, or analysis
  commands that may process large inputs, use `$agent-safe-local-runs`.
* Do not split Rust source into numbered bucket file series such as
  `parser01.rs` / `parser02.rs` or `part01.rs` / `part02.rs`. Module and file
  names must describe the responsibility or concept they own, especially when
  responding to code-metrics or complexity logs.
