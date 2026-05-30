# Hash Line Comments

Status: implemented

This record keeps the completed source comment spelling change after the
behavior moved into the specification. It is historical evidence, not the
source for current behavior.

## Read First

- Current source comments, documentation comments, doctests, and ADR-lite
  metadata: [../../specification/source-surface.md](../../specification/source-surface.md).
- Current formatter comment spelling:
  [../../specification/commands.md](../../specification/commands.md).
- Current editor fallback classification:
  [../../specification/editor-support.md](../../specification/editor-support.md).

## Implemented Boundary

`#` is the canonical ordinary line comment marker in Veln source. `##` is the
canonical documentation line comment marker. The lexer still accepts legacy
`//` and `///` comments for compatibility, and `veln fmt` canonicalizes legacy
comment spellings to hash comments when it can format the parsed source.

Documentation comments use `##` for doctest fences and ADR-lite metadata. In
`##` doctest fences, a leading `> ` line is hidden setup, while a visible
`# comment` line remains ordinary Veln source included in the generated
doctest. Legacy `///` doctest fences still accept the previous `# ` hidden
setup marker for compatibility.

## Completion Evidence

- Lexer and parser tests cover standalone, trailing, and documentation hash
  comments, including ADR-lite records.
- Doctest extraction tests cover `##` fences, `> ` hidden setup, and visible
  `#` comment lines inside examples.
- CLI JSON coverage checks that generated hash doctests typecheck through the
  command path.
- Formatter examples cover canonical standalone, trailing, and documentation
  hash comments.
- Specification examples use hash comments and `##` documentation comments for
  source-authored behavior.
- The executable source-surface grammar, editor semantic tokens, VSCode line
  comment configuration, and TextMate fallback grammar classify hash comments
  as comments.

## Non-Goals Preserved

- Do not add block comments.
- Do not change string, path, division, or operator parsing beyond comment
  tokenization.
- Do not add a second canonical documentation-comment spelling.
- Do not remove legacy `//` or `///` acceptance without a separate
  compatibility decision.

## Skip Unless Needed

- Do not read this page for current source syntax or formatter behavior.
- Use this record only when auditing why hash comments are canonical while
  legacy slash comments remain accepted.
