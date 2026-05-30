* Think in English.
* Do not write full path in files.
* Do not write calendar dates in documentation, source, comments, filenames,
  or metadata unless testing date-pattern behavior or preserving an externally
  defined identifier such as a URL.
* Treat `docs/specification/` as the source of current implemented
  behavior. Treat `docs/proposals/` as planned or accepted work only; do not
  cite or edit proposal text as current behavior unless the matching
  specification page also states it.
* When adding or changing human diagnostics, keep the primary message focused
  on the specific failed fact at the reported span. Put causes, provenance,
  repair hints, and other related locations in `related` notes, and add human
  output coverage when related context is expected.
* When adding or changing CI-visible messages, make the required action and why
  it matters clear; use `$ci-message-policy`.
