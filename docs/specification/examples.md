# Examples

Status: routing

Executable examples live under `../../examples/specification/`. Use that
directory's README and the focused `case.toml` files as the source of checked
behavior.

## Binary Schema Routes

- Schema-local helper projection:
  `../../examples/specification/run/binary-schema-local-projection-boundary/`.
- Parser rejection for schema-level `map to`:
  `../../examples/specification/check/schema-map-to-rejected/`.
- Same-module recursive dispatch helper decode and primitive-base encode:
  `../../examples/specification/run/binary-schema-recursive-dispatch-decode-encode/`.
- Same-module recursive dispatch missing primitive base rejection:
  `../../examples/specification/run/binary-schema-recursive-dispatch-rejected/`.

## Read When

- Updating executable specification coverage.
- Checking which public CLI behavior is pinned by a case.
