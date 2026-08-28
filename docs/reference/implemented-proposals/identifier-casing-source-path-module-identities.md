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
checked `identifier-casing-source-path-json`,
`identifier-casing-exported-source-path-json`,
`identifier-casing-source-path-import-isolation-json`,
`identifier-casing-source-path-duplicate-isolation-json`,
`identifier-casing-source-path-cycle-isolation-json`, and
`identifier-casing-source-path-human` examples fix the JSON and human command
diagnostics and the module-graph isolation boundary. The checked
`identifier-casing-source-path-boundary` example
fixes the LSP zero-width diagnostic range and source-path origin data
projection. The checked
`identifier-casing-chained-companion-boundary-json` example fixes the
combined invalid-casing and chained-companion structural boundary.

## Scope

Regular source paths validate the package-relative source path after removing
`.veln`. Exact `.test.veln` companions validate the target source path before
adding the internal companion suffix. Doctests validate the documented source
path before adding the doctest suffix and wrapper name. Generated sources with
origin module metadata validate the supplied origin segments before generated
bookkeeping paths or declaration names are considered. Generated sources
without origin module metadata do not introduce a source-visible module.
Chained companions do not validate synthetic recovery segments for casing;
they do not derive a source-visible module identity, and they keep the
existing `module.chained_companion` structural diagnostic boundary. Manifest
export path checks reuse the same accepted module
derivation boundary and report export origin casing failures as source-path
diagnostics. A regular source that is both selected normally and named by
`lib.exports` is classified once for source-path casing diagnostics, using
`source_kind: export`. A generated source selected by `lib.exports` keeps
generated origin metadata as the identity and casing authority, using
`source_kind: generated`; its generated bookkeeping path is not validated or
published as the exported module identity.

Each invalid origin segment emits `name.invalid_case` with `phase: name`,
`origin: source_path`, `occurrence: path_segment`, the segment spelling as
`name` and `segment`, `name_class: module`, `required_initial:
ascii_lowercase`, the observed initial class, `source_path`, `source_kind`,
and the zero-based `segment_index`. The primary span is zero-width at the
start of the affected source. The `source_kind` value is `regular`,
`export`, `companion`, `doctest`, or `generated`. A source with one or more
invalid origin segments is not registered as a normal derived module identity.
Its lowered declarations are not added to the normal source module graph and
cannot satisfy imports, collide with declarations in a valid source module, or
contribute dependency edges to cycle analysis.
Selected regular and companion sources still report source-path casing
diagnostics when the source also has parse diagnostics, but parse-failing
sources are not lowered or registered. A source path segment that starts with
an ASCII lowercase letter but contains another invalid module-identifier
character remains a structural `module.invalid_source_path` failure rather
than a source identifier casing failure.

## Completion

This slice is complete for source-path-derived module identities, including
generated-source origin metadata and manifest export paths. It does not
complete written module identity syntax, explicit import-alias syntax,
non-import qualified-use segment casing, recovery navigation, repair rename,
rename conflict prediction, or MCP rename mapping.
