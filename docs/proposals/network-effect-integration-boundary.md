# Network Effect Integration Boundary

Status: proposed

This proposal defines the boundary between a pure sans-I/O protocol core and
future transport integration. It is related to the HTTP/2 binary schema design
driver, but it should not block the first pure protocol-core slice.

## Problem

HTTP/2 eventually needs sockets, deadlines, timeouts, task scheduling, and
channels, but the design driver starts with a pure core. The language already
recognizes broad effect labels such as `net`, `time`, and `concurrency`, while
the proposal text raises finer-grained labels such as network read and write.

The project needs a clear boundary so binary schema work does not accidentally
commit to a full network runtime.

## Scope

Define future integration support for:

- mapping transport byte chunks into sans-I/O input events
- mapping outgoing chunks back to host transport writes
- use of `net`, `time`, and `concurrency` effects
- channel-first stream event routing
- per-stream task handling
- deadline and timeout vocabulary
- ownership of frame ordering, flow control, and transport writes

## Required Design Decisions

The proposal must resolve:

- whether broad `net` remains sufficient or access-mode labels are needed
- whether deadlines use `time` only or a richer timer API
- how transport errors map into protocol errors or host errors
- how stream handlers are exposed to application code
- how channel values interact with byte view freezing
- whether the first server example uses plain functions, stream tasks, or a
  small service interface

## Non-Goals

- Do not block the sans-I/O protocol core on sockets.
- Do not require TLS or ALPN.
- Do not change current implemented effect labels until runtime APIs require
  it.
- Do not define HTTP application routing.

## Completion Criteria

- Specification work distinguishes pure protocol functions from transport
  effectful functions.
- Examples show host-fed input chunks and outgoing chunks at the boundary.
- Effect inference and diagnostics cover any new compiler-known network or
  timer calls.
- The HTTP/2 design driver can remain pure while leaving a documented route to
  transport integration.
