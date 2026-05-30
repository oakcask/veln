# Hash Line Comments

Status: proposed

This page proposes changing the source line comment marker from `//` to `#`.
It is proposal work, not current language behavior unless `../specification/`
also states it.

## Read First

- Current source grammar, comments, documentation comments, doctests, and
  ADR-lite metadata:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current formatter behavior for standalone and trailing line comments:
  [../specification/commands.md](../specification/commands.md).
- Current editor fallback tokenization:
  [../specification/editor-support.md](../specification/editor-support.md).

## Current Boundary

The implemented lexer treats `//` through the end of the line as a comment.
Documentation comments are a convention over the same token shape: doc
comments start with `///`, and doctest extraction plus ADR-lite metadata strip
that prefix before reading Markdown fences or metadata fields.

The source grammar's executable Prolog model also treats `//` as trivia.
Formatter behavior preserves parsed comment text, attaches standalone comments
to nearby formatted source lines, and keeps trailing comments on the same
formatted line.

Executable doctests currently use a leading `# ` inside a `veln` fence as a
hidden setup marker. That marker is not a Veln source comment today, but it
would collide with ordinary visible source comments if `#` becomes the comment
marker.

## Target

Make `#` the canonical ordinary line comment marker in Veln source.
Documentation line comments should use `##`, preserving the existing rule that
doc comments are line comments with one extra marker character.

In the target source surface:

- `# comment` is a comment from `#` through the end of the line;
- `expr # comment` is a trailing line comment;
- `## doc` is a documentation line comment;
- `## ```veln` and adjacent `## ```veln-output ...` fences drive doctest
  extraction;
- `## @adr` and `## @adr-lite` start ADR-lite records;
- `//` and `///` are not canonical Veln comment markers.

## Proposed Rule

Add hash-prefixed line comment lexing and make it the formatter-emitted,
documentation-emitted, and example-authored spelling. The parser, formatter,
semantic analysis, command JSON, and editor support should keep the same
comment semantics after tokenization:

- comments remain trivia for parsing;
- standalone comments attach to the next parsed source line for formatting;
- trailing comments stay on the source line they trail;
- documentation comments still feed doctests and ADR-lite metadata;
- comments do not affect lowering, execution, reachability, effects, or type
  checking.

The documentation comment prefix should be `##`, not `###`. The current
`///` shape is "ordinary marker plus one extra marker"; carrying that relation
over gives the shortest doc-comment marker while leaving Markdown heading text
visible after stripping the prefix.

## Doctest Hidden Setup

Do not make `# ` serve both as a Veln source comment and as hidden setup inside
executable doctests. Before `#` becomes the canonical source comment marker,
introduce a replacement hidden setup marker that does not start with `#`.

Use a leading `> ` inside doctest fences:

```text
## ```veln
## > let seed = 1
## # visible example comment
## seed
## ```
```

The doctest extractor should remove the `> ` marker and include the rest of
that line in the generated test source. A visible `# comment` line inside a
doctest should remain visible example source and should be included as a
normal source comment.

This works because the current Veln source surface has no valid body,
declaration, pattern, or contract line whose first source token is `>`.
`>` remains a binary comparison operator inside expressions, but a source line
starting with `>` is already outside the valid surface. The hidden marker
should be exact after the documentation-comment prefix: `> ` at the beginning
of the doctest content, not "first non-whitespace `>`". That keeps ordinary
example indentation meaningful and avoids hiding an indented expression by
accident.

The existing `# ` hidden setup marker can be accepted during a transition while
`//` comments are still accepted, but it should not remain part of the final
target surface after `#` comments become canonical.

## Work Route

1. Add lexer support for `#` and `##` comments while keeping current `//` and
   `///` comments accepted as legacy spellings.
2. Add doctest extraction support for `##` doc comments and `> ` setup
   lines.
3. Update formatter golden tests so newly formatted files emit `#` and `##`
   while preserving source text only where lossless legacy preservation is
   explicitly required.
4. Update examples, fixtures, executable Prolog grammar, editor fallback
   grammar, semantic token expectations, and command/specification docs.
5. Add a targeted diagnostic or migration note for legacy `//` and `///`
   comments if the project wants a gradual deprecation path.
6. After examples and tests no longer require legacy comments, remove legacy
   `//` and `///` acceptance or keep it only behind a clearly documented
   compatibility mode.

## Migration Policy

Prefer a staged transition over a single breaking lexer change. A one-shot
switch would make examples, doctests, formatter snapshots, editor grammars,
and fixture coverage fail at once, and would make it harder to identify the
real semantic regressions.

During the compatibility stage, `veln fmt` should choose one of two deliberate
behaviors:

- canonicalize legacy `//` and `///` comments to `#` and `##`; or
- preserve legacy text only when lossless formatting already requires exact
  comment text preservation.

The implementation should choose one formatter behavior before accepting the
syntax change. Silent mixed-style output would increase generation variance and
make agent-authored examples less predictable.

## Non-Goals

- Do not add block comments.
- Do not change string, path, division, or operator parsing except that `//`
  stops being the canonical comment introducer.
- Do not change doctest metadata keys, expected-output fence semantics, or
  ADR-lite field names.
- Do not add a second documentation-comment form that remains canonical beside
  `##`.
- Do not promote this proposal into current behavior until the specification
  pages and examples are updated.

## Acceptance Checks

- The lexer recognizes standalone and trailing `#` comments as `Comment`
  tokens.
- The lexer recognizes `##` documentation comments and doctest extraction
  strips exactly the documentation prefix.
- Visible `# comment` lines inside doctest fences remain visible example
  source, while leading `> ` lines are hidden setup in generated test source.
- Negative doctests that intentionally demonstrate an invalid line starting
  with `>` have a documented escape route or are treated as outside the
  hidden-setup surface.
- ADR-lite records written with `## @adr` and `## @adr-lite` parse into the
  same structured metadata as the current doc-comment form.
- `veln fmt` has golden coverage for standalone, trailing, and documentation
  hash comments.
- The executable source-surface grammar models `#` comments as trivia.
- Editor semantic tokens and TextMate fallback rules classify hash comments as
  comments.
- Existing tests that do not intentionally cover legacy comment spelling keep
  passing after fixtures and examples are migrated.

## Open Questions

- Should `veln fmt` rewrite legacy comments automatically, or should a
  separate migration command handle source-wide rewriting?
- Should legacy `//` and `///` produce warnings, hard parse errors, or no
  diagnostics during the compatibility stage?
- Should generated documentation preserve the author's original comment
  marker during compatibility, or always render from the canonical `##`
  spelling?
- How long should `# ` hidden setup remain accepted after `> ` exists?
- Should a negative doctest be allowed to opt out of `> ` hidden setup when it
  intentionally demonstrates a line-start `>` parse error?

## Update When

- The lexer, formatter, doctest extractor, or editor grammar starts accepting
  hash comments.
- The project chooses a concrete legacy-comment deprecation policy.
- The current source specification is updated to make hash comments current
  behavior.
