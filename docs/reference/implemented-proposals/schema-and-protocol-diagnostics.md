# Schema And Protocol Diagnostics

Status: implemented

## Outcome

Schema, codec, and HTTP/2 failures project focused human diagnostics and
stable structured JSON details for byte offsets, field paths, readiness,
expected and actual facts, bounded byte previews, protocol state, limits, and
rule provenance. Pure protocol transitions retain typed error values; an
ordinary source-level projection function creates reportable diagnostics when
a caller chooses to report them.

Current command and JSON behavior is specified under
`../../specification/commands.md`,
`../../specification/diagnostics-json.md`,
`../../specification/run-json.md`, and
`../../specification/execution.md`.

## Evidence

Executable evidence lives in the binary-schema, codec, runtime-diagnostic,
and HTTP/2 cases under `../../../examples/specification/run/`. Focused records
in this directory retain completion evidence for individual diagnostic ids
and projection boundaries.

## Boundary

This record does not reserve an open-ended sequence of codec or protocol
diagnostic ids. A new failure shape should reuse an existing structured shape
when possible; a materially new shape needs a concrete behavior change and
focused executable evidence.
