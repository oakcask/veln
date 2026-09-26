---
role: proposal
update-when: The canonical veln-language failure contract or its checked scenario results change.
---

# Agent Language-Reference Skill Routing

## Remaining outcome

Align the canonical `veln-language` skill with the structured failure mapping
already required by its repository-local scenario harness. Each unavailable
search, unavailable topic, and stale snapshot entry must name the operation,
the recorded result kind and code, and the bounded-failure instruction. The
stale-snapshot mapping must also require the error URI to match the selected
snapshot URI.

After the alignment, the direct scenario replay and the complete
workflow-script test suite must pass. The implemented current behavior remains
owned by the agent-language-routing specification and its checked fixtures.

## Completion

This proposal is complete when all scenario and workflow-script checks pass
with the canonical skill. Remove this page and its Ready entry after that
evidence passes.
