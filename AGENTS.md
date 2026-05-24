* Think in English.
* Do not write full path in files.
* Do not write calendar dates in documentation, source, comments, filenames,
  or metadata unless testing date-pattern behavior or preserving an externally
  defined identifier such as a URL.
* When adding or changing human diagnostics, keep the primary message focused
  on the specific failed fact at the reported span. Put causes, provenance,
  repair hints, and other related locations in `related` notes, and add human
  output coverage when related context is expected.
