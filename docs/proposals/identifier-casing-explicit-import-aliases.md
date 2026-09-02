---
role: proposal
update-when: Explicit import-alias syntax, import alias lookup, module-class casing diagnostics, or the planned alias evidence changes.
---

# Explicit Import Alias Casing

## Summary

Require each source-written explicit import alias to start with an ASCII
lowercase letter. The alias is a module-class name and remains distinct from
the written import-path segments.

## Blocker

Veln has no explicit import-alias syntax. The executable grammar currently
accepts a `use` module path with an optional package source, and the final path
segment supplies the implicit alias.

This proposal is not selectable until an owning syntax proposal defines:

- the alias token and grammar position;
- the alias namespace, duplicate rule, and `prelude` collision behavior;
- lookup through the alias and its relationship to the imported path; and
- parser recovery for a missing or malformed alias token.

The owning syntax proposal may absorb this casing rule. If it does, remove
this page instead of creating two authorities for the same source surface.

## Casing Contract After The Blocker Clears

| Source state | Expected result | Planned evidence |
| --- | --- | --- |
| The explicit alias starts with an ASCII lowercase letter. | Accept the alias when the owning import rules otherwise accept it. | Accepted executable-grammar and `check` cases. |
| The explicit alias starts with an ASCII uppercase letter. | Report `name.invalid_case` at the alias token with name class `module`, then exclude the alias from normal lookup. | Exact JSON and human diagnostic cases with an alias-use cascade assertion. |
| The explicit alias token is underscore-led. | Report the same module-class casing failure at the retained token. | Parser-recovery and diagnostic cases. |
| The alias independently conflicts with another alias or the reserved `prelude` alias. | Preserve the independently provable duplicate or reserved-name diagnostic in deterministic order with the casing diagnostic. | Ordered overlap cases. |

Migration and source-carrier auditing apply only after the syntax exists and
only to repository-owned inputs that use that syntax.

## Completion

After the blocker clears, this proposal is complete when the four rows pass,
the executable grammar includes the syntax, and the import and diagnostic
specifications state the implemented behavior.
