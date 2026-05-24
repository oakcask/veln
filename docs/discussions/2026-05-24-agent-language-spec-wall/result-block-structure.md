# Discussion Result: Block Structure

Date: 2026-05-24

## Picked Question

- Should block structure use `do ... end`, indentation, braces, or a restricted
  combination?

## Decision

Use explicit keyword-delimited blocks in the first slice. A block-opening form
must be closed by `end`; indentation is formatting, not syntax; braces are not a
statement-block delimiter.

Use `do` only where it improves parsing clarity for expression-introducing
forms. Function bodies, `match` bodies, and other grammar heads that already
introduce a block may start the body directly and still close with `end`.

## Rationale

The first slice should make incomplete generated code easy to parse, diagnose,
and repair. An explicit `end` gives the parser a local recovery target and gives
agents a concrete missing-token diagnostic. Indentation-only syntax is compact,
but small whitespace edits can change nesting in ways that are harder to explain
in JSON diagnostics and generated patches. Braces are familiar to many tools,
but they add another high-variance style and move Veln toward punctuation-heavy
source without improving the repair loop.

Requiring `do` before every body would make block starts visually uniform, but it
adds noise where the grammar already has a clear block head. A restricted `do`
keeps the token available for ambiguous expression positions without making
every function or `match` carry another required keyword.

## First-Slice Rule

- Multi-line block forms close with `end`.
- Indentation is normalized by `veln fmt` and must not affect parse structure.
- `{ ... }` is reserved for records or future literal-like forms, not statement
  blocks.
- Missing `end` is a parser diagnostic with the unmatched block opener as
  related context when available.
- `do` is allowed only for grammar forms that need an explicit body separator;
  the exact list belongs in the first-slice grammar.
- The formatter owns canonical indentation for nested blocks so human and agent
  edits converge on one layout.

## Open Detail

The first-slice grammar now resolves the initial block heads for functions and
`match`. Future expression forms may still introduce `do` only when they need
an explicit body separator.

## Consequence

The first implementation can keep block parsing and recovery deterministic
while leaving enough room to refine compact forms and expression-level block
separators after examples expose real friction.
