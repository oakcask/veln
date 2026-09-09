---
name: toolchain-test-authoring
description: Add, update, or review Veln CLI integration cases under crates/veln-cli/tests/toolchain_cases/ and examples/specification/. Use when working with case.toml, case-text sidecars, generated toolchain cases, or harness assertions. Do not use for crate-only unit tests.
---

# Toolchain Test Authoring

## Goal

Produce a focused CLI integration case whose fixtures are readable, whose
assertions cover observable behavior, and whose generated harness test passes.
Use the repository's Veln MCP server while authoring the Veln source, but keep
the toolchain harness as the authority for command execution and expected
output.

## Authorities

- Read the smallest matching page under `docs/specification/` for current
  command or language behavior. Do not treat proposal text as current behavior.
- Read `docs/reference/toolchain-test-harness.md` when choosing manifest fields,
  sidecars, JSON assertions, protocol assertions, or fixture layout.
- Inspect nearby cases in the same command area before introducing a new
  assertion shape or layout convention.

## Workflow

1. Decide whether the behavior belongs in `examples/specification/` or
   `crates/veln-cli/tests/toolchain_cases/`. Put readable public CLI behavior in
   the former and low-level CLI or harness coverage in the latter.
2. Identify one observable behavior and the narrowest command invocation that
   demonstrates it. Extend a nearby case when that keeps one coherent behavior;
   otherwise create one case directory with one `case.toml` boundary.
3. Use Veln MCP during source authoring when its tools are available:
   - call `search_docs` or `read_doc` before inferring Veln syntax or standard
     library APIs;
   - pass an explicit root to `check_project` because the repository workspace
     normally contains many selected projects;
   - use `definition` or `references` when symbol identity is part of the case;
   - call `refresh_workspace` after adding, removing, or moving a `veln.toml`.
4. Keep MCP evidence in scope. `check_project` validates eligible saved Veln
   projects, but it does not interpret `case.toml`, execute the selected CLI
   command, compare streams, or validate runtime artifacts. A manifestless case
   may also be outside repository-root `check_project`; continue with the
   harness instead of treating that MCP boundary as a test failure.
5. Prefer existing manifest assertions and sidecar conventions. Keep protocol
   streams and substantial expected output in `case-text/` when that makes the
   behavior reviewable. Do not add a nested `case.toml`.
6. Run the narrow generated case through the guarded test entrypoint:

   ```text
   bash scripts/agent-test -p veln-cli --test toolchain_harness <case-name-filter>
   ```

   Use a stable generated-name prefix derived from the root and case path. If
   the filter is unclear, list tests first and then run the selected case in a
   separate command. Follow `$agent-safe-local-runs` for broader test scopes or
   commands that may consume substantial time or memory.
7. Expand verification only in proportion to the change. Harness parser or
   assertion implementation changes need focused harness unit coverage in
   addition to representative generated cases. Public behavior changes may
   also require the matching specification page and executable example to stay
   aligned.

## Completion

Report the case path, the behavior asserted, the narrow harness command and
result, and which MCP checks were used. Distinguish MCP validation from harness
evidence; do not claim command-level coverage from `check_project` alone.
