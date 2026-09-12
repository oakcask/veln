---
name: docs-progressive-disclosure
description: Change documentation structure, routes, classification, or metadata while preserving clear authority.
---

# Docs Progressive Disclosure

## Goal

Keep documentation discoverable through short routes with one clear authority
per subject.

## Authority

The normative policy is
[documentation-authoring.md](../../../docs/reference/documentation-authoring.md).
Read the relevant section when a change affects document classification,
metadata, placement, presentation, routing boundaries, or specification-writing
rules. A local prose correction that does not affect those concerns does not
require reading the policy or loading this skill.

## Workflow

- For a new or reclassified document, use the authoring policy to choose its
  authority, metadata, and location.
- For a move, split, merge, or route change, start from `docs/README.md` and the
  nearest affected README. Inspect subject boundaries, ownership, links, and
  same-stem `foo.md` / `foo-full.md` pairs.
- Keep routing documents focused on discovery. Put policy, behavior, and detail
  in the authoritative document selected by the route.
- Prefer focused, subject-named pages. When several pages form a distinct area,
  give that area a meaningful directory and a short README route.
- Do not create or preserve a summary/`*-full.md` pair for the same authority.
  Retire a touched same-scope pair, update its links, and do not leave
  routing-only compatibility files.
- Add or update required frontmatter for added, moved, reclassified, or
  substantially revised Markdown under `docs/`.
- Align affected prose with executable evidence, implementation, proposals, and
  historical records.
- Use `verifiable-specification-writing` for normative behavior.

## Verification

- Run the frontmatter checker for added, moved, reclassified, or substantially
  revised Markdown.
- Run the link checker and search for stale links when routes or links change.
- Inspect `update-when` selectors when ownership or update routing changes.
- Confirm that routing pages contain discovery information and point to the
  smallest current authority.
- Confirm that the change leaves no updated same-scope short/full pair, that
  merged or split content has one authority per subject, and that no internal
  link requires a deleted compatibility path.
- Confirm that the authoring policy, rather than a routing page or skill, owns
  any new or changed documentation rule.
