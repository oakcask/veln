# Drop Legacy Slash Comments

Status: implemented

This record keeps the completed removal of legacy slash comments after the
behavior moved into the specification. It is historical evidence, not the
source for current behavior.

## Read First

- Current source comments, documentation comments, doctests, and ADR-lite
  metadata: [../../specification/source-surface.md](../../specification/source-surface.md).
- Current formatter comment handling:
  [../../specification/commands.md](../../specification/commands.md).
- Historical hash-comment migration record:
  [hash-line-comments.md](hash-line-comments.md).

## Implemented Boundary

Veln source accepts only hash comments:

- `#` starts an ordinary line comment.
- `##` starts a documentation line comment.
- `//` is not an ordinary line comment marker.
- `///` is not a documentation line comment marker.

Documentation extraction uses only `##` comments. Executable doctests,
expected-output fences, and ADR-lite records no longer scan `///` lines.
Hidden doctest setup uses only `> `; the legacy `# ` hidden setup rule is
removed with the legacy `///` fence route.

## Completion Evidence

- Lexer coverage keeps `//` as two division tokens, and parser coverage treats
  slash-prefixed comment-like text as ordinary source that can diagnose.
- Parser documentation-comment collection only reads `##` comment tokens for
  ADR-lite records.
- Doctest extraction ignores `///` fences and keeps `##` fences for executable
  doctests, expected output, `> ` hidden setup, and visible `# comment` source.
- Formatter and CLI JSON coverage use canonical `#` and `##` comments and no
  longer migrate slash-prefixed comment-like text.
- The executable source grammar, editor semantic tokens, and VSCode TextMate
  fallback grammar classify only hash comments as comments.
- Specification examples include slash-prefixed comment rejection through the
  public `check --json` command.

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
- `../../specification/source-surface.md`,
  `../../specification/commands.md`,
  `../../specification/test-json.md`, and
  `../../specification/editor-support.md` describe only the implemented
  post-removal behavior after the code changes land.

## Skip Unless Needed

- Do not read this page for current source syntax or formatter behavior.
- Use this record only when auditing why slash-prefixed comment compatibility
  was removed.
