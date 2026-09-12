# Commit Message Bodies

Use a body when the subject alone cannot explain the reason, implications, or
important tradeoff. Keep it concise, usually one to three short paragraphs.

A useful body covers only the relevant points:

- the important behavior, contract, or decision change
- why the change is necessary and why the chosen approach fits
- why a likely alternative or adjacent change is excluded
- the consequence or risk future maintainers need to know

Do not list changed files or functions unless their identity explains the
decision.

Example:

```text
The parser now treats empty input as a valid no-op so callers can pass optional
configuration without pre-filtering. Malformed non-empty input remains an
error, preserving the boundary callers use to detect invalid configuration.
```
