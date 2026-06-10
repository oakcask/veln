# Binary Schema Primitives And Dispatch

Status: proposed

This proposal defines the binary-schema field vocabulary needed for frame
headers and frame-specific payload dispatch. It depends on a schema declaration
surface and a byte standard-library vocabulary.

## Problem

HTTP/2 frame decoding needs more than ordinary records. A frame header contains
non-byte-aligned semantic fields, endian-sensitive integers, flags, reserved
bits, and a payload whose interpretation depends on a tag value. These are
external representation facts, not internal Veln type declarations.

## Scope

Define binary schema support for:

- exact-width unsigned fields such as 8-bit, 24-bit, and 31-bit values
- endian-aware field reads and writes
- reserved bits that are consumed but not exposed as ordinary data
- flags that decode as raw bits, bitsets, or frame-specific ADTs
- length-prefixed payloads
- field references inside later field definitions
- dispatch from a tag field to payload schemas
- unknown tags that preserve raw payload bytes when permitted
- schema-level structural validation

## Required Design Decisions

The proposal must resolve:

- whether exact-width names belong to schema primitives or source types
- how reserved bits are spelled
- how field references are scoped and type checked
- how dispatch handles unknown or extension frame types
- how much dependent structure is allowed before a schema becomes a parser
  language
- how schema-generated values map to independently declared ADTs and records

## Non-Goals

- Do not encode HTTP/2 stream-state legality in schema declarations.
- Do not require HPACK support.
- Do not define network effects or task scheduling.
- Do not optimize binary layout.

## Completion Criteria

- Examples show a binary frame header schema with fixed widths and reserved
  bits.
- Examples show tag-based payload dispatch and unknown tag preservation.
- Invalid fixed fields and truncated fields produce structured diagnostics.
- The schema vocabulary is general enough for another binary protocol example.
- The HTTP/2 design driver can express frame header and payload boundaries
  without ordinary parsing functions doing all layout work.
