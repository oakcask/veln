---
role: specification
authority: normative
update-when: The CLI top-level help or subcommand help behavior changes.
specification-coverage: usage=#command-help; behavior=#command-help; limits=#command-help
---

# Command Help

Top-level help is printed for an empty invocation, `veln --help`, `veln -h`,
and `veln help`. Subcommand help is printed by `veln help <command>` and
by `--help` or `-h` before the command-specific argument separator.

For `run`, help flags after `--` are entry arguments. Unknown help topics and
extra topic arguments are command-line errors. Help writes human text to stdout
and performs no discovery, parsing, checking, lowering, compilation, execution,
repair, or JSON emission.

## References

CLI parser tests in `crates/veln-cli/src/cli/tests.rs` cover global,
subcommand, separator, unknown-topic, and extra-topic behavior.
