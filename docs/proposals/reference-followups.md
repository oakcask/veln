# Reference Follow-Ups

Status: proposed

This page collects follow-up work that remains outside current specification
behavior. Proposal text is not current behavior unless `../specification/`
also states it.

## Read First

- Current behavior: [../specification/README.md](../specification/README.md).
- Proposal index: [README.md](README.md).
- Implemented formatter follow-up record:
  [formatter-stabilization.md](formatter-stabilization.md).

## Follow-Up Targets

No formatter follow-up target is active on this page. The earlier formatter
comment-attachment and stabilization target is implemented and routed through
[formatter-stabilization.md](formatter-stabilization.md). New formatter work
needs a new proposal page before this follow-up index can select it.

- Broader executable reachability, entry selection, and runtime behavior not
  yet stated by [../specification/execution.md](../specification/execution.md).
- Additional test discovery, example extraction, and test event reporting not
  yet stated by [../specification/test-json.md](../specification/test-json.md).
- Repair application workflows beyond advisory hole diagnostics and safe
  candidate records.
- Backend replacement work covered by
  [jvm-bytecode-backend.md](jvm-bytecode-backend.md).
- Self-hosting standard library expansion covered by
  [self-hosting-standard-library.md](self-hosting-standard-library.md).

## Update When

- Move a target into `../reference/` only after current code and tests support
  it.
- Remove a target from this page when the matching specification page fully states
  the implemented behavior.
- Keep implemented records only when they route useful history or completion
  evidence without restating current behavior.
- Keep remaining proposed implementation work in this page or the matching
  short proposal page.
