---
role: reference
authority: supporting
update-when: The dependency schema-alias chain resolver, bounded graph tests, or their representative test commands change.
---

# Dependency Schema-Alias Chain Performance Audit

This audit records the bounded performance check for the direct-dependency
schema-alias chain slice. The workload is the generated shared-suffix and
disconnected-cycle graph at 64, 128, and 256 aliases. The test asserts the
visit counter remains bounded while resolving the selected alias.

The original recorded executable-case run, before the final evidence-only
fixture adjustment, took 1.24 seconds for the LSP case and 1.92 seconds for
the MCP case (test-body times; the commands reused the already-built test
binary). The new run after the adjustment took 1.22 seconds and 1.90 seconds,
respectively. These are comparative observations, not CI thresholds; the
full wall times included incremental compilation and were 20.59 seconds and
18.49 seconds.

The main hot path is `resolved_schema_alias_chains` in `veln-sema`: it builds
module-local alias/schema indexes once, then walks each alias through the
finite chain with a path-position map and memoized suffix results. The
visited-identity guard rejects cycles without rescanning the graph. The
adjacent blocker case completed in 0.02 seconds in the test body and returned
the successful empty result for ineligible chains.

Representative commands and results:

- `bash scripts/agent-test -p veln-language-service --lib dependency_schema_alias_shared_suffix_and_disconnected_cycle_stay_bounded` — passed; generated sizes 64, 128, and 256.
- `bash scripts/agent-test -p veln-language-service --lib dependency_schema_alias_chain_intermediate_blockers_return_empty` — passed; adjacent negative case.
- `bash scripts/agent-test -p veln-cli --test toolchain_harness examples_specification_lsp_references_dependency_schema_alias` — passed.
- `bash scripts/agent-test -p veln-cli --test toolchain_harness examples_specification_mcp_references_dependency_schema_alias` — passed.

The relevant `veln-language-service` tests and the paired LSP/MCP toolchain
cases therefore provide both the asymptotic guard and the representative
adapter workload. No stable wall-time budget is asserted.
