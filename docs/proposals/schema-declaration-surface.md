# Schema Declaration Surface

Status: proposed

This proposal defines the source syntax needed to declare schemas as external
representation boundaries. It is a prerequisite for the HTTP/2 binary schema
design driver because that driver needs a source-visible way to describe frame
header layout before codec execution or protocol state rules can be tested.

## Problem

Current source syntax has functions, tests, ADT type declarations, records,
contracts, effects, imports, and public aliases. It does not have a top-level
declaration for an external representation boundary.

The HTTP/2 design driver needs a declaration that can say:

- a field is read from bytes rather than from an internal Veln value
- a field has a fixed external width
- a field is validated at the schema boundary
- a field may map into an independently declared Veln type
- a schema reports structural failures with field paths and byte positions

Without a schema declaration, binary protocol examples must encode external
layout in ordinary functions, which hides the boundary the driver is meant to
exercise.

## Scope

Define source support for:

- top-level `schema` declarations
- named schema fields
- field type annotations that may name schema primitives
- field-local validation clauses
- mapping from schema fields to Veln values
- schema visibility and module ownership rules
- parser, AST, formatter, editor token, and documentation behavior

## Required Syntax Decisions

The proposal must resolve:

- whether `schema` is a top-level declaration beside `type` and `fn`
- whether binary schemas use `schema` directly or a specialized
  `codec schema` form
- how schema field validation is spelled
- how schema field values are mapped into source ADTs or records
- whether schema declarations generate both decoders and encoders
- how schema declarations are imported, exported, and referenced

## Non-Goals

- Do not define the complete binary primitive vocabulary here.
- Do not implement HTTP/2 protocol state rules here.
- Do not require a network runtime.
- Do not treat schemas as aliases for internal Veln types.

## Completion Criteria

- The accepted grammar includes schema declarations.
- Parser, AST, formatter, and editor support understand schema declarations.
- Examples show schema declarations as boundary contracts, not ordinary types.
- Diagnostics distinguish malformed schema syntax from failed schema
  validation.
- The HTTP/2 design driver can express its frame header boundary without using
  placeholder text syntax.
