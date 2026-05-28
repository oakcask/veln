# Formatter Stabilization Completion Review

This review records the completion gate for the selected formatter
stabilization proposal. It is review evidence, not current command behavior.

## Read First

- Proposal record:
  [../proposals/formatter-stabilization.md](../proposals/formatter-stabilization.md).
- Current formatter behavior:
  [../specification/commands.md](../specification/commands.md) and
  [../specification/commands-full.md#veln-fmt](../specification/commands-full.md#veln-fmt).
- Current comment and source syntax:
  [../specification/source-surface.md](../specification/source-surface.md).

## Result

The formatter stabilization target is implemented and promoted into the command
specification. The formatter attaches standalone comments to formatted module
headers, imports, function signatures, contract clauses, body lines, and
closing `end` lines. Trailing comments remain attached to their source line.

Parser recovery around formatter-owned layout now accepts comment-separated
imports and contract clauses through the same newline-tolerant declaration
paths used by ordinary source parsing.

## Verification

- `cargo test -p veln-syntax format_tree_attaches_comments_to_imports_contracts_and_end_lines`
- `cargo test -p veln-cli --test check_json fmt_attaches_comments_to_imports_contracts_and_end_lines`

## Boundaries

Do not use this review to add execution, test discovery, repair workflows,
backend replacement, or standard library behavior. Those remain separate
proposal areas unless the current specification already states them.
