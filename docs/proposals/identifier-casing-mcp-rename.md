---
role: proposal
update-when: The MCP tool surface adds rename, shared rename failure codes change, or planned MCP rename casing evidence changes.
---

# MCP Rename Casing Mapping

## Summary

Map shared class-preserving rename failures to a future MCP rename tool without
creating transport-specific identifier-casing exceptions.

## Blocker

The current MCP surface exposes workspace discovery, refresh, project
diagnostics, and definition. It does not expose rename, prepare-rename, edit
results, or rename failures. The agent-language-services proposal also keeps
MCP mutation and rename outside its current bounded capability.

This proposal is not selectable until an owning MCP rename proposal defines:

- the checked input and result schemas;
- supported symbol and source scopes;
- edit and no-edit success results;
- domain failure representation; and
- saved-snapshot capture and state-preservation behavior.

## Mapping Contract After The Blocker Clears

| Shared rename result | Expected MCP result | Planned evidence |
| --- | --- | --- |
| A selected symbol rejects a class-changing replacement with `rename.invalid_case`. | Return the owning MCP rename contract's failure shape with the same shared code and no edits. | Checked schema and MCP stdio cases. |
| A selected symbol rejects a predictable conflict with `rename.conflict`. | Return the owning MCP rename contract's failure shape with the same shared code and no edits. | Checked MCP conflict-mapping cases. |
| A recovery or module segment is outside the supported MCP rename surface. | Return the owning contract's unsupported or empty result without edits; do not reinterpret the segment as another name class. | Checked unsupported-selection cases. |

## Completion

After the blocker clears, this proposal is complete when all three mappings
pass through the checked MCP schemas and stdio harness and the MCP
specification states the implemented failure boundary.
