---
role: routing
update-when: The agent-language-services lifecycle review artifact, frozen-inventory target provenance, or lifecycle validator command changes.
---

# Agent Language Services Lifecycle

Use this route when checking the lifecycle review artifacts for the later
frozen-inventory bootstrap.

## Artifacts

- Reviewed source decisions:
  [../agent-language-services-lifecycle-review/source-decisions.json](../agent-language-services-lifecycle-review/source-decisions.json).
- Frozen-inventory target provenance:
  [provenance.json](provenance.json).
- Local validator:

```text
node workflow-scripts/check-agent-language-services-lifecycle.mjs validate
```

The reviewed source decisions are the pre-inventory authority. The later
frozen-inventory implementation may consume them but must not rewrite them.
