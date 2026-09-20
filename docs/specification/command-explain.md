---
role: specification
authority: normative
update-when: The veln explain command listing, diagnostic lookup, or output behavior changes.
specification-coverage: usage=#explain-command; behavior=#explain-command; limits=#explain-command
---

# Explain Command

Use `veln explain <DIAGNOSTIC_ID>` to print a diagnostic title, meaning, and
repair-oriented note. Use `veln explain --list` to print the IDs in the
implemented catalog. The command is read-only: it does not discover, parse,
check, lower, compile, or run source.

An invocation must provide exactly one of `--list` or a diagnostic ID.
Missing, unknown, or extra topics are command-line errors. The catalog currently
includes hole, contract-predicate, and satisfy-parser diagnostics such as
`hole.unfilled`, `hole.satisfy_type_mismatch`, `parse.contract_predicate`,
and `parse.satisfy_predicate`; `--list` is authoritative when the catalog
changes.

## References

The command is implemented in `crates/veln-cli/src/commands/explain.rs`.
CLI validation tests cover list, lookup, missing-topic, unknown-topic, and
extra-topic failures.
