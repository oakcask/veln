# HTTP/2 Sans-I/O Protocol Core

Status: proposed

This proposal defines the actual HTTP/2 protocol-core slice used by the binary
schema design driver. It depends on schema declarations, binary schema
primitives, byte vocabulary, codec execution, diagnostics, and binary fixture
helpers.

## Problem

The design driver needs a concrete protocol target, but the target should not
be a production Web server. The useful slice is a sans-I/O core that accepts
byte input events, returns outgoing byte chunks, and models protocol state with
ordinary Veln values.

## Scope

Define a small HTTP/2 core covering:

- connection preface validation
- frame header decode and encode
- SETTINGS
- PING
- GOAWAY
- DATA
- HEADERS with opaque header-block payloads
- CONTINUATION handling only as needed to keep header-block boundaries valid
- typed protocol errors
- connection settings
- stream identifiers
- stream lifecycle
- inbound and outbound flow-control windows
- graceful shutdown state

## Required Design Decisions

The proposal must resolve:

- which frame validation rules are schema-level structural failures
- which frame validation rules are protocol-state failures
- how stream identifiers and flow-control counters are typed
- how header-block continuation state is represented
- how unknown frame types are preserved or ignored
- how SETTINGS changes affect later frame-size validation
- how HPACK is represented as an opaque or minimal codec boundary

## Non-Goals

- Do not implement TLS, ALPN, socket listeners, or platform networking.
- Do not require complete HPACK support.
- Do not optimize for production throughput.
- Do not encode all protocol state rules inside schema declarations.

## Completion Criteria

- Examples show valid and invalid frame fixtures for the target slice.
- A pure decode state transition handles chunk arrival and end-of-stream.
- Protocol-state failures are typed and diagnostically structured.
- The core keeps only undecoded suffix bytes after frame consumption.
- HPACK has a reserved boundary for later work.
- The design driver can use the core to evaluate schema, byte, codec,
  diagnostic, and standard-library decisions.
