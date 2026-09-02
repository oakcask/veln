---
role: implementation-record
authority: supporting
update-when: Source-path-derived module identity casing evidence, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Source Path Module Identities

## Outcome

Source-path-derived module identity segments now use the source identifier
casing diagnostic contract for module-class path segments. Current behavior is
specified by [Name Resolution](../../specification/name-resolution.md),
[Check JSON And Diagnostics](../../specification/diagnostics-json.md), and
[Editor Support](../../specification/editor-support.md). Package
documentation catalog behavior for exported source-path casing failures is
specified by
[Package Documentation Catalogs](../../specification/package-documentation.md).
The checked
`identifier-casing-source-path-json`,
`identifier-casing-exported-source-path-json`, and
`identifier-casing-source-path-human` examples fix the JSON and human command
diagnostics. The checked
`identifier-casing-mixed-dependency-export-json` example fixes the direct
dependency mixed-export boundary: an invalid-cased exported source path
reports export-provenance source-path casing, contributes no normal public
module identity, does not satisfy import or qualified-use lookup through
dependency recovery, and does not prevent a valid sibling export from being
analyzed. The checked `identifier-casing-source-path-boundary` example
fixes the LSP zero-width diagnostic range and source-path origin data
projection, and checks that the source-path diagnostic range is not an LSP
rename target. The checked
`identifier-casing-chained-companion-boundary-json` example fixes the
combined invalid-casing and chained-companion structural boundary.
The checked `identifier-casing-source-path-import-isolation-json`,
`identifier-casing-source-path-duplicate-isolation-json`, and
`identifier-casing-source-path-cycle-isolation-json` examples fix the
registration and graph consequences. An invalid source-path-derived identity
does not satisfy a local import, does not become a duplicate participant, and
does not add a reachable module-graph edge. Focused surface-analysis coverage
checks the same isolation boundaries.

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
In a direct dependency manifest, this export boundary is per export for
source-path casing failures. A dependency with one invalid-cased exported
source path and one valid lowercase sibling keeps the valid sibling export
visible to imports and language-service dependency snapshots. The invalid
exported identity is omitted from the public export set.

Each invalid origin segment emits `name.invalid_case` with `phase: name`,
`origin: source_path`, `occurrence: path_segment`, the segment spelling as
`name` and `segment`, `name_class: module`, `required_initial:
ascii_lowercase`, the observed initial class, `source_path`, `source_kind`,
and the zero-based `segment_index`. The primary span is zero-width at the
start of the affected source. The `source_kind` value is `regular`,
`export`, `companion`, `doctest`, or `generated`. A source with one or more
invalid origin segments is not registered as a normal derived module identity.
Selected regular and companion sources still report source-path casing
diagnostics when the source also has parse diagnostics, but parse-failing
sources are not lowered or registered. A rejected derived-module identity is
recorded only when visible module derivation itself fails solely with
source-path `name.invalid_case` diagnostics. A parse-failing source whose
source path casing is accepted does not record a rejected derived-module
identity and does not suppress a single-segment unresolved local import. A
source path segment that starts with an ASCII lowercase letter but contains
another invalid module-identifier character remains a structural
`module.invalid_source_path` failure rather than a source identifier casing
failure.

## Completion

This slice is complete for source-path-derived module identities, including
generated-source origin metadata, manifest export paths, and LSP source-path
module rename exclusion. Transport-independent package documentation catalog
failure for exported source-path casing is specified separately as package
documentation behavior. This slice does not complete written module identity
syntax, explicit import-alias syntax, test dependency selection, partial
artifact analysis, or MCP rename mapping. Recovery navigation and source
declaration or binding recovery rename are completed separately in
[Identifier Casing Recovery Navigation And Rename](identifier-casing-recovery-navigation.md).
Language-service navigation isolation for sources with invalid
source-path-derived module identities is completed separately in
[Identifier Casing Source Path Navigation Isolation](identifier-casing-source-path-navigation-isolation.md).
LSP rename conflict rejection for valid selected workspace symbols is
completed separately in
[Identifier Casing Rename Conflicts](identifier-casing-rename-conflicts.md).
Qualified-use path casing is completed separately in
[Identifier Casing Qualified Use Paths](identifier-casing-qualified-use-paths.md).
