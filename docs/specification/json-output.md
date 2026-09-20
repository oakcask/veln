---
role: routing
update-when: A command JSON specification or its subject boundary changes.
---

# JSON Output

Choose the page for the command whose output you consume. Each page owns its
envelope, field meanings, ordering, and failure behavior.

| Command or subject | Specification |
| --- | --- |
| `check --json`, shared diagnostic objects, human diagnostic alignment | [Check JSON and diagnostics](diagnostics-json.md) |
| `run --json`, captured output, runtime failure projections | [Run JSON](run-json.md) |
| `test --json`, selection, cases, events, and suite results | [Test JSON](test-json.md) |
| `repair --json`, preview, apply, refusal, and verification records | [Repair JSON](repair-json.md) |
| `metrics --json`, policy checks, and baseline documents | [Metrics JSON](metrics-json.md) |

For source selection, command options, and execution gates, use
[commands.md](commands.md). For source-level HTTP/2 behavior behind a runtime
diagnostic projection, use [http2.md](http2.md).
