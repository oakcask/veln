# Package Export Manifest Surface

Status: implemented

This record covers the manifest/export slice of file-based modules and
packages. Current behavior lives in `../../specification/`; this page is only
history and completion evidence.

## Implemented Behavior

- `veln.toml` rejects `[modules]`.
- `[lib].exports` accepts package-relative `.veln` source-file paths.
- Export paths must stay inside the package, use file-path spelling rather
  than module-path spelling, derive valid source module paths, match selected
  source files, and avoid duplicate derived module exports.
- Manifest exports do not add files to the selected source set and do not
  rename source modules.
- Local same-package module/import behavior remains path-derived and unchanged.

## Current Specification

- Source and manifest surface:
  `../../specification/source-surface.md`.
- Shared command manifest validation:
  `../../specification/commands.md`.
- Name and module behavior:
  `../../specification/names-effects.md`.
- Human and JSON diagnostics:
  `../../specification/diagnostics-json.md`.

## Executable Evidence

- `../../../examples/specification/check/manifest-exports/`.
- `../../../examples/specification/doc/manifest-modules-rejected/`.
- `../../../crates/veln-cli/tests/toolchain_cases/check/manifest-modules-rejected-json/`.
- `../../../crates/veln-cli/tests/toolchain_cases/check/manifest-unselected-boundary-json/`.
