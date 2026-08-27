---
name: docs-progressive-disclosure
description: Use when creating, reorganizing, splitting, merging, or maintaining documentation so agents and humans can start from short indexes and read only the task-relevant files.
---

# Docs Progressive Disclosure

## Goal

Apply the repository documentation policy while keeping task routes short and
details progressively disclosed.

## Authority

Read
[documentation-authoring.md](../../../docs/reference/documentation-authoring.md)
before changing documentation. That document is the normative source for
classification, metadata, placement, presentation, routing boundaries, and
specification-writing rules. Do not copy those rules into routing pages or this
skill.

## Workflow

1. Start from `docs/README.md` and select the route that matches the task.
2. Read the authoring policy and the README for each affected documentation
   area.
3. Inspect the relevant routes, subject boundaries, maintenance ownership, and
   same-stem `foo.md` / `foo-full.md` pairs before moving, merging, or splitting
   content.
4. Classify every added, moved, or substantially changed document under the
   authoring policy.
5. Keep routing documents focused on discovery. Move policy, behavior, and
   detail into the authoritative document selected by the route.
6. Split a broad document into focused, subject-named pages. Add or update the
   nearest directory README to route to each page, and introduce a meaningful
   subdirectory when several related pages need their own scope.
7. Do not create a summary/`*-full.md` pair for the same authority. If the
   change updates either member of an existing same-scope pair, retire the pair
   in the same change. Merge a single subject into one document. For multiple
   subjects, create focused pages under a meaningful hierarchy. Delete the
   superseded paired files and update their links; do not leave routing-only
   compatibility files.
8. Add or update required frontmatter for every Markdown document changed under
   `docs/`.
9. Update relative links after moves and align affected prose with executable
   evidence, implementation, proposals, and historical records.
10. Use `verifiable-specification-writing` when the change creates,
   substantially revises, or reviews normative behavior.
11. Run the verification steps and resolve failures before reporting completion.

## Verification

- Search for stale links after moving or renaming documents.
- Inspect extracted update selectors with
  `rg -n '^update-when:' docs -g '*.md'`.
- Run `node workflow-scripts/check-doc-frontmatter.mjs` with the changed
  Markdown paths.
- Run `node workflow-scripts/check-doc-links.mjs` when routes or links change.
- Confirm that routing pages contain discovery information and point to the
  smallest current authority.
- Confirm that the change leaves no updated same-scope short/full pair, that
  merged or split content has one authority per subject, and that no internal
  link requires a deleted compatibility path.
- Confirm that the authoring policy, rather than a routing page or skill, owns
  any new or changed documentation rule.
