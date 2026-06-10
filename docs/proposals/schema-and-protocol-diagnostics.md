# Schema And Protocol Diagnostics

Status: proposed

This proposal defines diagnostics for schema, codec, and protocol-state
failures. It is a prerequisite for the HTTP/2 binary schema design driver
because byte-level failures must be repairable by agents and distinguishable
from stream-state protocol errors.

## Problem

Existing diagnostics cover source parsing, type checking, effects, contracts,
holes, commands, and runtime failures. Binary schema and HTTP/2 protocol work
needs additional structured context:

- byte offsets
- schema field paths
- expected widths and lengths
- actual available byte counts
- decoded tag values
- related settings or configured limits
- connection and stream state at protocol failure sites

Without this shape, tests can only match broad error strings and agents cannot
repair fixtures or implementations reliably.

## Scope

Define diagnostic support for:

- schema structural failures
- codec incomplete-input reports
- codec invalid-input reports
- integer conversion overflow at byte boundaries
- schema field paths
- byte offsets and bounded-buffer offsets
- related notes for settings, limits, and source of protocol rules
- protocol-state failures as peer errors or implementation contract failures

## Required Shape Decisions

The proposal must resolve:

- canonical diagnostic ids for schema and codec failures
- how byte offsets appear in human output and JSON output
- how field paths are represented
- how incomplete input differs from invalid input in command output
- how protocol errors carry stream id, frame kind, and current state
- when related notes are required rather than primary-message detail

## Non-Goals

- Do not define full binary schema syntax.
- Do not implement HTTP/2 state machines.
- Do not replace existing source, type, effect, or contract diagnostic shapes.

## Completion Criteria

- Human and JSON examples cover truncated input and invalid fixed fields.
- Protocol-state examples cover invalid frame kind for a connection or stream
  state.
- Diagnostics keep the primary message focused on the failed fact at the
  reported span or byte position.
- Related notes carry provenance, settings, limits, and state-transition
  context.
- The HTTP/2 design driver can test valid and invalid binary fixtures with
  stable diagnostic assertions.
