---
role: reference
authority: normative
update-when: The proposal Ready and Blocked catalog, proposal target metadata schema, readiness manifest, or target-validation workflow registration changes.
---

# Proposal Target Readiness

Generated proposal targets are valid only when the selected proposal or
heading is listed under Ready in `docs/proposals/README.md` and the readiness
manifest names the same state at the declared base commit. The validator reads
the catalog, proposal frontmatter, prerequisite pages, and completed records
from that commit. Later changes in the implementation branch cannot complete a
prerequisite retroactively.

Run the local check with:

```sh
node workflow-scripts/check-proposal-target-readiness.mjs validate
```

Every generated `TARGET.md` handoff has an adjacent `TARGET.json` sidecar.
Validate the sidecar before writing or implementing the Markdown target:

```sh
node workflow-scripts/check-proposal-target-readiness.mjs validate prompts/TARGET.json
```

The metadata schema is `target.schema.json`. Target metadata records the
proposal path, heading anchor, default branch, full hexadecimal base commit,
prerequisites, and target kind. The manifest is the tracked authority for Ready
and Blocked entries that generated targets may select. The base commit is a full
commit identity on the declared default branch, and the implementation branch
must have that exact merge base.
