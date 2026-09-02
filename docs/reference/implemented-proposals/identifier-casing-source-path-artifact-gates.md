---
role: implementation-record
authority: supporting
update-when: Identifier casing source-path artifact-gate behavior, command specifications, or executable source-path artifact examples change.
---

# Identifier Casing Source Path Artifact Gates

## Outcome

`run` and `test` reject selected source-path-derived invalid module identities
before JVM classfile artifact generation and Java launch.

The current behavior is specified by [Run Command](../../specification/command-run.md),
[Test Command](../../specification/command-test.md), and
[Name Resolution](../../specification/name-resolution.md).

## Scope

This record completes the JVM artifact-command slice of the identifier-casing
acceptance row for invalid derived modules beside artifact consumers. It does
not complete export consumers, deferred recovery consumers, explicit import
aliases, or MCP rename mappings.

## Evidence

| Command | Selected source state | Required result | Evidence |
| --- | --- | --- | --- |
| `run` | The explicit run input has a source-path-derived module segment with invalid casing. | Report `name.invalid_case`, return the diagnostic envelope, create no JVM cache artifact, start no Java launcher, and produce no user-code output. | `examples/specification/run/identifier-casing-source-path-artifact-gate-json/` |
| `run` | An invalid-cased source-path sibling is outside the explicit run input set. | Do not report the sibling diagnostic and run the selected entry normally. | `examples/specification/run/identifier-casing-source-path-unselected-artifact-json/` |
| `test` | The explicit selected test source has a source-path-derived module segment with invalid casing. | Report `name.invalid_case`, mark the discovered selected case `blocked` with `reason: "static_gate"`, create no JVM cache artifact, start no Java launcher, and produce no user-code output. | `examples/specification/test/identifier-casing-source-path-artifact-gate-json/` |
| `test` | An invalid-cased test source sibling is outside the selected test analysis set. | Do not report the sibling diagnostic and run the selected case normally. | `examples/specification/test/identifier-casing-source-path-unselected-artifact-json/` |

The blocking cases use the harness fake Java launcher marker as negative
launch evidence and assert that the JVM cache artifact root is absent.
