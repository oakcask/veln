---
role: implementation-record
authority: supporting
update-when: Source-path-derived module identity language-service navigation isolation, sibling identifier-casing completion boundaries, or current specification authority for this record changes.
---

# Identifier Casing Source Path Navigation Isolation

## Outcome

Workspace sources whose source path derives an invalid-cased module identity
are isolated from normal language-service navigation. Current behavior is
specified by [Editor Support](../../specification/editor-support.md) and
[Name Resolution](../../specification/name-resolution.md). The checked
`identifier-casing-source-path-boundary` LSP example covers diagnostics,
definition, references, prepare-rename, rename, qualified selections through
the invalid identity, edit-free rename results, and continued navigation in an
unrelated valid module. Focused `veln-language-service` tests cover the same
snapshot and open-document overlay isolation boundary.

## Scope

The implemented slice covers shared language-service and LSP definition,
references, prepare-rename, and rename. A source with an invalid
source-path-derived module identity does not contribute source declarations,
source declaration recovery records, binding recovery records, or classified
qualified path segments to the normal workspace navigation index. Positions in
that source return no selected symbol. Positions in other sources return no
selected symbol when the selection would resolve a module-qualified type,
constructor, function, or function-value use through the invalid module
identity. Rename for those unsupported selections returns no workspace edits,
and prepare-rename returns no range.
References and rename edits for valid symbols selected from other sources omit
occurrences inside the invalid source identity.

The isolation is local to the invalid source identity. It does not make source
identifier casing diagnostics a workspace-global navigation gate. Unrelated
valid workspace sources continue to provide normal definition, references,
prepare-rename, and rename behavior.

## Completion

This slice is complete for language-service navigation isolation of
source-path-derived invalid module identities in captured workspace snapshots
and open-document overlays. It does not complete explicit import-alias syntax,
MCP rename mapping, export generation, documentation generation, backend
artifact handling, or other artifact-command consumers.
