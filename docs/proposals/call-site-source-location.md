---
role: proposal
update-when: Generated-source mapping, relocation-safe source identity, deferred-observation lifetime coverage, or call-site lowering changes.
---

# Call-site Source Location

Libraries that produce diagnostics, logs, events, and traces need the user's
call site rather than the source location inside a wrapper function. Veln must
provide this without making each observability function a compiler-recognized
special case.

The location must be captured when the library API is called. A trace can be
finished or exported after the originating call stack no longer exists, so a
later stack walk cannot recover the required logical call site.

The declaration, static-checking, and direct- and indirect-call runtime
behavior are specified in
[Call-site Declarations](../specification/call-site-declarations.md). This
proposal tracks only the remaining S6, S7, and S9 source-identity and lifetime
work below.

The remaining source-identity work must make `package` and `module`
disambiguate equal relative paths from different dependencies and must verify
that relocation preserves those identities.

## Generated and Virtual Sources

An origin path identifies a source but does not map positions into that source.
A generated-source origin map must associate generated boundaries with the
original source path and its line, column, and byte offset. The source model
must retain those associations through semantic lowering.

When both boundaries of a call expression have origin entries, its
`SourceLocation` uses the original source path and the mapped start and end
coordinates. The implementation must resolve the file and all six coordinates
as one location. It must not combine an original path with generated
coordinates.

When either boundary is unmapped, including when a generated source has only an
origin path, the complete `SourceLocation` uses the generated or virtual source
identifier and its coordinates. This fallback prevents a partially mapped
value from naming the wrong coordinate space.

The file field uses the same canonical virtual-source naming contract as
diagnostics. Moving a package to another machine must not change the exposed
file value.

### Mapping model boundary

The first implementation needs one original source per generated source and
explicit mappings for the start and end boundaries that a reported span uses.
It does not infer original positions from equal numeric offsets or from an
origin path. It does not need a serialized source-map format, range
interpolation, multiple original files in one generated source, or composed
multi-stage maps.

The generated-origin data must survive source loading, surface AST wire
round-trips, and semantic lowering. A lookup must either return one complete
original `SourceSpan` or preserve the complete generated `SourceSpan`.

## Acceptance Model

| Case | Source form | Required observation | Planned evidence |
| --- | --- | --- | --- |
| S6 mapped | Generated and original call expressions have different paths and all six coordinates differ. Explicit origin entries cover both generated boundaries. | The exposed file, start, and end equal the independently constructed original span. No field comes from the generated span. | Source-model lookup and wire-round-trip tests plus a generated-source JVM runtime fixture whose expected value comes from the original source. |
| S6 fallback | A generated call expression has only an origin path, no origin metadata, or a mapping that omits either boundary. | The exposed file and all six coordinates equal the generated span. No field comes from the partial origin metadata. | Source-model boundary tests and mapped, path-only, partial, and unmapped runtime cases. |
| S7 | Equivalent packages under two absolute roots contain dependencies with the same package-relative source path. | Exposed `file` values are package-relative or canonical virtual paths, all fields contain neither root and remain identical after relocation, and `package` plus `module` disambiguate the dependency sources. | Relocation and dependency-collision test. |
| S9 | A trace retains a `callsite` value after its originating function returns. | Later observation reports the captured location without walking the current stack. | Deferred-observation run case. |

## Verification and Promotion

Remaining implementation must extend generated-source mapping and lifetime
coverage.

## Related Work

[ECMA-426](https://tc39.es/ecma426/) represents each decoded mapping with a
generated position and an original source position. This supports the design
conclusion that a source identifier alone cannot establish an original
coordinate. Veln adopts the separation between generated and original
positions, but not the ECMA-426 wire format, UTF-16 coordinate convention, or
implicit lookup behavior. Veln retains its Unicode-scalar columns and byte
offsets and requires explicit mappings for both reported span boundaries.

[Veln PR #1736](https://github.com/oakcask/veln/pull/1736) explored replacing
the generated file with an origin path while retaining generated coordinates.
That result showed why path metadata and a position map must be separate: the
value named one source while its coordinates addressed another. This proposal
retains the useful unmapped fallback from that work, rejects the mixed
coordinate-space result, and keeps S6 planned until executable evidence uses
an independent original span as its oracle.

## Non-goals

- This proposal does not expose a runtime stack trace.
- This proposal does not make a runtime-required predicate in an ordinary
  function construct call-site context for a call-site-aware callee. Only an
  enclosing call-site-aware function has hidden context that its predicate can
  forward.
- Source locations are not stable identifiers across source edits.
- The proposal does not add general optional or default parameters.
- The proposal does not add syntax that overrides implicit call-site context at
  an individual call expression.
- The proposal does not expose an arbitrary current expression's location as a
  `SourceLocation` value.
- The proposal does not give libraries access to machine-specific source
  paths.
