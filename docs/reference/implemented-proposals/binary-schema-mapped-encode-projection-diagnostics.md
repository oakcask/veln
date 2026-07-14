# Binary Schema Mapped Encode Projection Diagnostics

Status: implemented

This record preserves the completed mapped encode projection diagnostic slice
from `binary-schema-primitives-and-dispatch.md`. Current
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`,
`../../specification/diagnostics-json.md`, and checked executable examples
under `../../../examples/specification/`.

## Outcome

When a binary schema's structural `map to Target` decode mapping is accepted
but generated encode cannot project a mapped target field back to the visible
schema-local fields required by `byte_encode_<schema>`, the checker reports
`schema.mapping_encode_projection` at the blocking mapping assignment instead
of falling back to a generic derived helper eligibility failure.

The diagnostic names the schema, mapping target, mapping target path, target
value path, expected generated encode helper, unavailable helper direction,
and projection fact. Related notes keep schema declaration, derive encode
request, and helper-boundary context outside the primary message.

Nested dispatch payload helper diagnostics keep using `schema.dispatch_payload`
for parent payload eligibility, but now share the same projection fact for
nested schemas whose decode helper is available and encode helper is not.

## Evidence

This historical slice was retired with schema-level mapping support.

## Remaining Work

The broader binary schema proposal remains open for schema value mapping and
other helper slices outside the implemented structural mapping and diagnostic
boundaries.
