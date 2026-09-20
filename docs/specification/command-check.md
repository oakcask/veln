---
role: specification
authority: normative
update-when: The veln check command gate, source discovery behavior, dependency loading, or output contract changes.
specification-coverage: usage=#check-command; behavior=#discovery-and-ownership; limits=#limits-and-errors
---

# Check Command

Use `veln check [PATH ...]` to analyze source. With no paths, it recursively
selects owned regular `.veln` files below the selected package root. A
directory path is searched recursively; a file path selects that file. The
final list is sorted and deduplicated. With `--json`, the command emits the
check diagnostic envelope; otherwise it prints diagnostics or `ok`.

## Discovery and ownership

Discovery does not follow source or directory symbolic links and skips `.git`
directories. A regular descendant `veln.toml` starts a nested package and
excludes that directory and descendants from outer discovery. A symlink or
non-regular object named `veln.toml` is not a boundary. `target` has no
special source meaning.

Explicit files and directories must remain under the selected root, must not
cross a nested package boundary, and must not escape through parent paths or
symbolic links. One invalid input fails the complete discovery operation.
Boundary-classification errors also fail; the command does not silently omit
the path.

## Manifest and dependencies

When a manifest is present, `check` validates package/tool metadata,
dependencies, and `[lib].exports`. Path, vendor, mirror, and already
materialized git dependencies may be loaded for imports. Git entries require a
remote and exactly one of `rev`, `tag`, or `branch`; `subdir` selects a
package root within the repository. Source analysis does not fetch, clone,
resolve revisions, update checksums, or write a lockfile.

An export is a package-relative `.veln` file path, not a module path. It must
match a selected source, derive a valid module identity, avoid test companions,
and not duplicate a derived module. `[modules]` is rejected. An imported
dependency must have a direct regular `veln.toml`, matching package name, and
an exported imported module; only public declarations and aliases cross that
boundary.

## Analysis gates

Parse diagnostics suppress semantic diagnostics for that file, but other
parse-clean files may still report semantic errors. Cross-file imports and
qualified calls from parse-clean files participate in the same analysis path as
`run` and `test`. Lowering reports checked-core blockers such as missing
expressions and call or constructor arity mismatches.

## Limits and errors

Discovery is atomic for the invocation: a rejected input produces no partial
success report. Dependency sources must already be available; the command does
not perform dependency acquisition.

## References

Implementation: `crates/veln-cli/src/commands/check.rs`. Discovery and
dependency boundary coverage is under `examples/specification/check/` and
`crates/veln-cli/tests/check_json/check.rs`.
