# Proposals

Status: routing

This directory contains only planned or incomplete work. Proposal text is not
current language behavior unless the matching page under
`../specification/` also states it.

## Read First

- Current behavior: [Language Specification](../specification/README.md).
- Completed proposal history:
  [Implemented Proposal Records](../reference/implemented-proposals/README.md).

## Catalog

### First Experiment

- [Lexical Operation Handlers](lexical-operation-handlers.md): add nominal
  operation effects and automatically resuming lexical handlers without
  exposing continuations.
- [HTTP/2 Duplex Stream Connection Driver](http2-duplex-stream-connection-driver.md):
  drive one HTTP/2 client or server connection through the proposed abstract
  duplex-stream effect and an existing TCP `NetStream` handler.

Read these proposals in order. The connection-driver proposal depends on the
lexical-handler proposal.

### Follow-Ups After Evidence

- [One-Shot Resumable Effect Handlers](one-shot-resumable-effect-handlers.md):
  add explicit deep, one-shot resumptions only after a checked example shows
  that automatic resumption cannot express the required control flow.
- [Effect-Polymorphic HTTP/2 Services](effect-polymorphic-http2-services.md):
  add listener ownership, per-connection tasks, and application callbacks
  whose effects are preserved by the service API.

Do not select either follow-up before its activation gate is met. The first
experiment is intended to provide the evidence for those gates.

## Selection Rule

Before implementing a proposal slice, compare it with the matching
specification page and executable cases. Do not select work that is already
covered there or only extends a numbered, width-based, arity-based, route-count,
or diagnostic-id sequence. Such work needs a concrete new capability and a
bounded stopping condition.

## Proposal Shape

Express observable targets as structured acceptance cases, decision tables,
state-transition tables, executable models, or another directly verifiable
form when practical. Map those targets to the tests, fixtures, doctests,
benchmarks, or executable specifications that will verify implementation.
Keep prose for scope, rationale, non-goals, and constraints that cannot
reasonably be expressed in the primary verification medium. Do not describe
planned evidence as already passing.

State proposed behavior declaratively as an externally observable contract.
Keep internal algorithms and ordered procedures outside normative behavior
unless they are required design constraints. Use Simplified Technical English
style when those details need prose explanation.

## Update When

Promote observable behavior to executable evidence under
`../../examples/specification/` first when practical, then update the smallest
matching specification page. Move completed proposal history to
`../reference/implemented-proposals/` and remove it from this catalog.
