---
role: specification
authority: normative
update-when: The CLI top-level help or subcommand help behavior changes.
---

# Command Help

Top-level help is printed for an empty invocation, `veln --help`, `veln -h`,
and `veln help`. Subcommand help is printed for `veln help <command>` and for
`--help` or `-h` before the command-specific argument separator.

For `run`, help flags after `--` are entry arguments, not command help flags.
Unknown help topics and extra help-topic arguments are command-line errors.

Help invocations emit human help text on stdout and do not discover, parse,
check, lower, compile, run, repair, or emit command JSON.

