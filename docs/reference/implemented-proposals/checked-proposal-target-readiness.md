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
- The documentation-validation workflow runs the readiness manifest check.
- The readiness tests cover accepted Ready targets and rejected blocked,
  unlisted, malformed, duplicate, no-target, and catalog-drift cases.

## Boundary

The validator checks selection readiness. It does not decide whether the
selected proposal's implementation evidence passes after the implementation
branch is complete.
