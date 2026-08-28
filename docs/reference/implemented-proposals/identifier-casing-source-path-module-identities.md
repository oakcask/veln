---
role: implementation-record
authority: supporting
update-when: Source-path-derived module identity casing evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Source Path Module Identities

## Outcome

Source-path-derived module identity segments now use the source identifier
casing diagnostic contract for module-class path segments. Current behavior is
specified by [Name Resolution](../../specification/name-resolution.md) and
[Check JSON And Diagnostics](../../specification/diagnostics-json.md). The
checked `identifier-casing-source-path-json` and
`identifier-casing-source-path-human` examples fix the JSON and human command
diagnostics. The checked `identifier-casing-source-path-boundary` example
fixes the LSP zero-width diagnostic range.

## Scope

Regular source paths validate the package-relative source path after removing
`.veln`. Exact `.test.veln` companions validate the target source path before
adding the internal companion suffix. Doctests validate the documented source
path before adding the doctest suffix and wrapper name. Chained companions do
not validate synthetic recovery segments for casing; they keep the existing
structural diagnostic boundary. Manifest export path checks reuse the same
accepted module derivation boundary.

Each invalid origin segment emits `name.invalid_case` with `phase: name`,
`origin: source_path`, `occurrence: path_segment`, the segment spelling as
`name` and `segment`, `name_class: module`, `required_initial:
ascii_lowercase`, the observed initial class, `source_path`, `source_kind`,
and the zero-based `segment_index`. The primary span is zero-width at the
start of the affected source. A source with one or more invalid origin
segments is not registered as a normal derived module identity.

## Completion

This slice is complete for existing source-path-derived module identities. It
does not complete generated-source origin metadata, written module identity
syntax, explicit import-alias syntax, non-import qualified-use segment
casing, recovery navigation, repair rename, rename conflict prediction, or
MCP rename mapping.
