---
role: proposal
update-when: The proposal Ready and Blocked catalog, generated target metadata, prerequisite lifecycle, default branch, or target-validation command contract changes.
---

# Checked Proposal Target Readiness

## Summary

Reject a generated implementation target unless its proposal path and section
are selectable from the Ready catalog at the target's declared base revision.
This repository-maintenance check turns the existing selection rule into a
finite validation result before implementation work begins.

## Problem

The proposal catalog and agent instructions already prohibit selecting a
blocked target. Generated prompt files are ignored, so CI cannot observe a
target that violates that rule. A target can therefore name a blocked section,
and a later correction loop can try to complete its prerequisite inside the
same stack. That stack no longer has one reviewed base state: an early commit
can create an artifact before its prerequisite, while a later PR uses that
artifact-bearing branch as its base and disables a bootstrap guard.

More prose in a blocked proposal does not close this gap. Target issuance needs
one machine-readable handoff that binds the selected proposal, section,
prerequisites, and base revision before the implementation branch is created.

## Scope

Add a repository-maintenance command that validates generated target metadata.
The metadata contains exactly these fields:

- repository-relative proposal path;
- proposal heading anchor;
- default-branch name;
- exact base commit;
- zero or more prerequisite proposal paths; and
- target kind: `proposal`, `proposal-section`, or `no-target`.

Add a tracked readiness manifest that enumerates every Ready and Blocked page or
section with its exact proposal path, heading anchor, state, and prerequisite
proposal paths. The validator compares the manifest with both catalog sections
in both directions. Target metadata is not the prerequisite authority; its
prerequisite array must equal the selected manifest entry.

The command reads the proposal catalog and Markdown frontmatter at the declared
base commit. A proposal target is valid only when all these facts hold:

- the path exists under `docs/proposals/` and has `role: proposal`;
- the path or exact heading link appears in the Ready section;
- it does not appear in the Blocked section;
- the prerequisite paths equal the readiness manifest entry, and every path is
  absent from `docs/proposals/` and has a linked completed implementation
  record;
- the declared base commit is on the declared default branch;
- the working branch merge base equals the declared base commit; and
- a target-specific phase precondition, when present, accepts that base.

A `no-target` result is valid only when the Ready section has no bounded
candidate after current specification and executable evidence are checked. It
does not authorize a Blocked entry.

Target generation invokes this command before writing an implementation
handoff. Agent guidance routes target generation and target review through the
same command. The command exits unsuccessfully and does not emit an accepted
handoff when any fact fails.

## Checked Handoff Contract

Define checked JSON Schemas for the readiness manifest and target metadata, and
a semantic validator for repository state. Run the same acceptance and
rejection corpus through both structural validators where their fields overlap.
The semantic validator uses Git object reads for the declared base; it does not
trust the current working tree's proposal catalog or readiness manifest.

The failure output names the rejected proposal path and heading, the failed
readiness or prerequisite fact, and the required next action. A blocked target
directs the maintainer to select the named Ready prerequisite. A stale base
directs the maintainer to regenerate the target from the current default
branch. A phase-precondition failure names the required base state.

The local command accepts an explicit metadata path so checked fixtures do not
depend on ignored prompt files. A thin target-generation integration may pass
generated target prompt metadata to the same validator, but prompt parsing is
not a second authority.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select a ready proposal page. | A role-correct page listed only under Ready at the declared base is accepted. | Positive fixture with an exact base commit and proposal path. |
| Select a ready subsection. | An exact Ready heading link is accepted; an unlisted heading in the same page is rejected. | Paired listed-heading and adjacent-heading fixtures. |
| Select blocked work. | A page or subsection listed under Blocked is rejected even when its prose describes a bounded first step. | Blocked-page and blocked-subsection rejection fixtures. |
| Omit or invent a prerequisite. | A target prerequisite array that differs from the readiness manifest is rejected, and a catalog entry missing from the manifest is rejected. | Missing, extra, duplicate, and stale prerequisite fixtures plus bidirectional catalog-manifest coverage. |
| Bypass a prerequisite in one stack. | A target whose prerequisite still exists at the declared base is rejected; adding its completion later in the working branch does not change the result. | Base-object fixture with a working-tree-only prerequisite removal. |
| Use a stale or unrelated base. | A base outside the declared default branch or a mismatched merge base is rejected. | Stale-base, side-branch-base, and mismatched-merge-base fixtures. |
| Violate a target phase precondition. | A target-specific checker rejects the handoff before its diff allowlist runs. | Frozen-inventory fixture whose base already contains a frozen artifact or lacks a completed prerequisite. |
| Emit no target. | `no-target` is accepted only when no Ready bounded candidate remains. | Empty-Ready acceptance and nonempty-Ready rejection fixtures. |
| Mutate handoff structure. | Schema and semantic validation both reject every missing field, extra field, invalid target kind, non-relative path, empty anchor, malformed commit, and duplicate prerequisite. | Shared schema/semantic mutation corpus. |

## Non-Goals

- Implementing or completing any selected product proposal.
- Deciding whether proposal acceptance evidence passes after implementation.
- Making ignored prompt files part of CI input.
- Inferring prerequisites from arbitrary prose instead of declared proposal
  links and catalog state.
- Allowing a target to close its own prerequisite in the same branch stack.

## Completion Rule

This proposal completes only when all nine acceptance rows pass, target
generation invokes the checked command, and the command has a documented local
route. Move the completed record out of `docs/proposals/` and remove it from
the Ready catalog after implementation.
