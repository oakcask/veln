# Self-Hosting Standard Library

Status: proposed

This page routes the active source-backed standard library proposal work.
Implemented standard symbols, effects, compiler-known calls, helper semantics,
and already source-backed helpers live in
[../specification/names-effects.md](../specification/names-effects.md).

## Current Target

Move one remaining descriptor-only pure prelude helper into the existing
source-backed helper model. The proposal changes helper source placement and
descriptor metadata only; implemented signatures, value semantics, and the
current source-backed split remain specification material.

Read these in order for the next helper:

- current helper behavior and descriptor-only versus source-backed split:
  [../specification/names-effects.md](../specification/names-effects.md)
- current source syntax available for the embedded helper body:
  [../specification/source-surface.md](../specification/source-surface.md)
- migration pattern and candidate filter:
  [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates)

## Read First

- Use [../specification/names-effects.md](../specification/names-effects.md)
  first for all current standard symbol behavior.
- Use
  [self-hosting-standard-library-full.md#remaining-pure-helper-candidates](self-hosting-standard-library-full.md#remaining-pure-helper-candidates)
  only after the specification identifies a descriptor-only helper whose
  behavior is already implemented.

## Read When

- Moving another descriptor-only helper into the source-backed pure-helper
  model.
- Deciding which helpers must remain compiler-known until source-level effects
  or runtime boundaries are specified.
- Checking whether a standard-library idea is current behavior or still future
  proposal work.

## Candidate Rule

A valid target is a pure prelude helper that already has an implemented
signature and value semantics, is still descriptor-only, can be written with
current source syntax, and needs no new effects, runtime boundary, parser
feature, module loading, source-level effect handler, streaming, subprocess, or
container representation guarantee.

## Skip Unless Needed

- Do not use this page for current standard symbol behavior or helper
  semantics.
- Do not open the full proposal when the current target and candidate rule
  above answer the task.
