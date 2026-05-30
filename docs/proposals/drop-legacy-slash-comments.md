# Drop Legacy Slash Comments

Status: proposed

This proposal removes compatibility support for legacy slash comments from
Veln source. Proposal text here is not current language behavior unless
`../specification/` also states it.

## Read First

- Current source comments, documentation comments, doctests, and ADR-lite
  metadata: [../specification/source-surface.md](../specification/source-surface.md).
- Current formatter comment handling:
  [../specification/commands.md](../specification/commands.md).
- Historical hash-comment migration record:
  [../reference/implemented-proposals/hash-line-comments.md](../reference/implemented-proposals/hash-line-comments.md).
- Proposal promotion checks:
  [implementation-route.md](implementation-route.md).

## Current Boundary

`#` is the canonical ordinary line comment marker and `##` is the canonical
documentation line comment marker. The lexer still accepts `//` ordinary line
comments and `///` documentation line comments as legacy spellings. `veln fmt`
rewrites those legacy spellings to `#` and `##` when the parsed source can be
formatted.

Doctest extraction, expected-output fences, and ADR-lite metadata currently
accept `///` documentation comments in addition to canonical `##`
documentation comments. Legacy `///` doctest fences also accept `# ` as hidden
setup, while canonical `##` doctest fences use `> ` for hidden setup and keep
`# comment` lines as visible source comments.

## Target

Veln source accepts only hash comments:

- `#` starts an ordinary line comment.
- `##` starts a documentation line comment.
- `//` is not an ordinary line comment marker.
- `///` is not a documentation line comment marker.

Documentation extraction uses only `##` comments. Executable doctests,
expected-output fences, and ADR-lite records no longer scan `///` lines.
Hidden doctest setup uses only `> `; the legacy `# ` hidden setup rule is
removed with the legacy `///` fence route.

## Work Route

- Update the lexer and executable source grammar so `//` and `///` are not
  comment tokens.
- Preserve `/` as the division operator and keep division diagnostics
  unrelated to comment removal.
- Update parser documentation-comment collection so only `##` tokens provide
  documentation text.
- Update doctest extraction and ADR-lite parsing to ignore slash-prefixed
  lines as documentation comments.
- Remove formatter migration from `//` to `#` and from `///` to `##`; after
  this target, `veln fmt` formats parse-clean canonical source only.
- Remove editor fallback highlighting for `//` comments while keeping semantic
  comment tokens for canonical comments.
- Update specification examples, command examples, and toolchain cases so
  source-authored examples use only `#` and `##`.

If a targeted diagnostic is added for slash-prefixed comment-like text, the
primary message should state the failed fact at the slash span. The repair hint
or related note should say to use `#` for ordinary comments or `##` for
documentation comments.

## Non-Goals

- Do not add block comments.
- Do not add a second canonical documentation-comment spelling.
- Do not change string, path, division, operator, or URL text parsing beyond
  removing slash-comment tokenization.
- Do not keep `# ` as hidden doctest setup after legacy `///` doctest fences
  are removed.
- Do not make formatter responsible for migrating source that no longer parses
  as valid Veln.

## Acceptance Checks

- Lexing and parsing reject or diagnose standalone and trailing `//` comment
  text as non-comment source.
- Lexing and parsing reject or diagnose `///` documentation comment text as
  non-comment source.
- Canonical `#` standalone and trailing comments still parse and format.
- Canonical `##` documentation comments still extract doctests, expected
  output, and ADR-lite metadata.
- `##` doctests still treat `> ` as hidden setup and keep `# comment` as
  visible source.
- `///` doctest and ADR-lite examples no longer produce generated doctests,
  expected-output expectations, or ADR-lite records.
- Formatter, CLI JSON, editor, and executable grammar coverage no longer rely
  on legacy slash comments.
- `../specification/source-surface.md`,
  `../specification/commands.md`,
  `../specification/test-json.md`, and
  `../specification/editor-support.md` describe only the implemented
  post-removal behavior after the code changes land.

## Update When

- Promote implemented behavior into `../specification/` only after parser,
  formatter, doctest, command, editor, and executable grammar coverage all
  agree on hash-only comments.
- Move the completed record to `../reference/implemented-proposals/` when the
  legacy slash compatibility path is fully removed.
