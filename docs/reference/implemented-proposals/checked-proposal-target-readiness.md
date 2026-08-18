---
role: implementation-record
authority: supporting
update-when: The proposal target readiness validator, manifest, metadata schema, or workflow registration is superseded.
---

# Checked Proposal Target Readiness

The proposal target-readiness handoff is implemented as a repository
maintenance validator.

## Completion Evidence

- The tracked readiness manifest and schemas start at
  [Proposal Target Readiness](../proposal-target-readiness/README.md).
- The local command is:

```sh
node workflow-scripts/check-proposal-target-readiness.mjs validate
```

- Generated target metadata can be checked by passing its JSON path to the
  same command.
- `AGENTS.md` rejects Markdown-only target handoffs. Generated targets require
  a checked `TARGET.json` sidecar before implementation begins.
- The documentation-validation workflow runs the readiness manifest check.
- The readiness tests use temporary Git histories. They cover an accepted
  Ready base and reject blocked, unlisted, malformed, duplicate, no-target,
  catalog-drift, nonexistent-base, stale-merge-base, and working-tree-only
  prerequisite completion cases.

## Boundary

The validator checks selection readiness against the declared base commit. It
does not decide whether the selected proposal's implementation evidence passes
after the implementation branch is complete.
