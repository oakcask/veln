---
role: reference
authority: normative
update-when: The proposal Ready and Blocked catalog, proposal target metadata schema, readiness manifest, or target-validation workflow registration changes.
---

# Proposal Target Readiness

Generated proposal targets are valid only when the selected proposal or
heading is listed under Ready in `docs/proposals/README.md` and the readiness
manifest names the same state.

Run the local check with:

```sh
node workflow-scripts/check-proposal-target-readiness.mjs validate
```

To validate a generated target handoff, pass its JSON metadata path:

```sh
node workflow-scripts/check-proposal-target-readiness.mjs validate path/to/target.json
```

The metadata schema is `target.schema.json`. The manifest is the tracked
authority for Ready and Blocked entries that generated targets may select.
