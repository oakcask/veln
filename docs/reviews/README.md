---
role: routing
update-when: A review evidence record is added, removed, or reclassified.
---

# Reviews

This directory keeps bounded review evidence for proposal slices and local
quality gates. These records support review decisions. They are not current
behavior specifications.

## Read When

- [toolchain-analysis-stage-benchmark.json](toolchain-analysis-stage-benchmark.json)
  records the controlled stage-timing benchmark for the bounded toolchain
  analysis proposal.
- [toolchain-analysis-reachable-lookups.json](toolchain-analysis-reachable-lookups.json)
  records the controlled comparison for indexed reachable and semantic lookup
  candidates.
- [toolchain-analysis-demand-standard-library.json](toolchain-analysis-demand-standard-library.json)
  records the controlled comparison for demand-driven embedded
  standard-library initialization.
- [toolchain-analysis-separated-standard-inputs.json](toolchain-analysis-separated-standard-inputs.json)
  records the controlled comparison for separate application and selected
  standard-library analysis inputs.
- [toolchain-analysis-embedded-lowered-standard.json](toolchain-analysis-embedded-lowered-standard.json)
  records the controlled comparison for embedded lowered standard-library
  modules.
- [toolchain-analysis-separated-reachable-inputs.json](toolchain-analysis-separated-reachable-inputs.json)
  records the controlled comparison for separated application and selected
  standard-library reachable-entry lowering inputs.
- [metrics-similarity-benchmark.json](metrics-similarity-benchmark.json)
  records the controlled comparison for metrics whole-body similarity
  workloads before and after partial source analysis.
- A proposal or reference page links to a named review record.
- A local benchmark result or audit record is needed to check why a proposal
  slice was accepted.

Use `../specification/` for current behavior and `../proposals/` for remaining
planned work.
