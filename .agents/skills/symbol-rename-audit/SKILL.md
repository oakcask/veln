---
name: symbol-rename-audit
description: Use when renaming, removing, or replacing symbols, fields, exported types, JSON keys, CLI output keys, metric names, diagnostics names, or compatibility aliases. Ensures agents search broadly, classify residual names, and do not report completion while old names remain unreviewed.
---

# Symbol Rename Audit

## Goal

Make rename and removal work verifiable. A rename is not complete just because the build passes or the edited call sites compile; the old and new names must be searched, and every residual hit must be classified.

## Workflow

1. Write a small migration map before editing:
   - old name
   - new name or removal target
   - affected surface: source, tests, docs, public JSON, exported TypeScript, CLI output, generated samples, or workflow output
   - compatibility policy: remove, keep as alias, historical docs only, or follow-up
2. Search for exact old names and type-name variants before and after editing.
3. Search for broader prefixes when the old name family is patterned, such as `phase11`, `Phase11`, `phase13V`, or a common suffix.
4. Search current names too, so tests and docs prove the new primary path exists and is not only added beside unreviewed old paths.
5. Split search results by ownership surface:
   - implementation code
   - test code
   - public contract or schema tests
   - CLI or generated output shape
   - current reference docs
   - phase, review, or other historical docs
6. Classify every residual old-name hit as one of:
   - `removed`: should no longer appear
   - `compatibility-alias`: intentionally emitted or exported for consumers
   - `historical-doc`: describes past phases, old reviews, or migration history
   - `test-assertion`: asserts absence, alias equality, or migration behavior
   - `follow-up-required`: outside the current PR scope but not safe to ignore
7. Update tests according to the compatibility policy:
   - Removed public fields need negative assertions.
   - Compatibility aliases need equality or shape assertions proving they point to the same data as the current name.
   - New primary names need positive assertions in the relevant public contract, CLI, or schema tests.
8. Do not report the rename as complete until there are no unclassified residual old-name hits in the touched surfaces.

## Search Guidance

Prefer `rg` for working tree searches and `git grep` when inspecting a branch or commit.

Search examples should be adapted to the actual migration map:

```sh
rg -n '\b(oldName|OldName|oldPrefix[A-Z][A-Za-z0-9_]*)\b' packages docs
rg -n '\b(newName|NewName)\b' packages docs
git grep -n -E '\b(oldName|OldName)\b' branch-name -- packages docs
```

For JSON field removals, include string-literal searches such as:

```sh
rg -n '"oldFieldName"|Object\.hasOwn\(.*"oldFieldName"' packages docs
```

Do not rely on a broad prefix search alone. It is useful for discovery, but it will include allowed historical and compatibility references that still need classification.

## Completion Report

When finishing a rename or removal, summarize:

- the migration map
- the search patterns used
- the residual old-name classification
- the tests or checks that enforce the intended contract
- any follow-up-required hits

Avoid saying "complete" if the only evidence is compilation, linting, or tests without residual-name classification.

## PR Description Guidance

For PRs that change public contracts, generated output, exported types, or CLI schemas, include a short compatibility note:

```markdown
## Compatibility

- Removed: `oldField`
- Current path: `newField`
- Remaining old-name hits: compatibility aliases in tests and historical docs only
```

If old names intentionally remain, say where and why. If follow-up work remains, name it directly instead of implying the rename is finished.
