# Proposals

This directory catalogs planned or accepted work that is not fully documented
as current behavior under `../specification/`. Proposal text is not current
language behavior unless the matching specification page also states it.

Use this page as a catalog only. Pick the proposal that matches the task, then
compare it with `../specification/` before changing behavior.

## Catalog

- [Implicit Prelude And Unqualified Imports](implicit-prelude-and-unqualified-imports.md):
  make `use` introduce unambiguous public exports into bare scope, define the
  standard prelude as an implicit import, and rename `core_prelude` to
  `prelude`.
- [Public Member Alias Re-Exports](public-member-alias-reexports.md):
  add explicit public aliases for re-exporting `fn` and `type` members without
  writing wrappers or exposing implementation module paths.

## Update When

- New proposal work is added, split, superseded, completed, or removed.
- Proposal work becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A completed proposal record moves to
  `../reference/implemented-proposals/`.
