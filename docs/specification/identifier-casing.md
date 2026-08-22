---
role: specification
authority: normative
update-when: Source-written declaration or binding casing validation, `name.invalid_case`, or the checked identifier-casing examples change.
---

# Identifier Casing

Source-written type and constructor names start with an ASCII uppercase letter.
Source-written function and value-binding names start with an ASCII lowercase
letter.

| Name class | Required initial | Covered source positions |
| --- | --- | --- |
| Type | `A` through `Z` | Source ADT declarations and public type aliases. |
| Constructor | `A` through `Z` | Public and private source ADT variants. |
| Function | `a` through `z` | Function declarations, test declarations, and public function aliases. |
| Value binding | `a` through `z` | Parameters, result bindings, local `let` bindings, match and destructuring bindings, handler context parameters, operation-clause parameters, and `satisfy` candidates. |

The rule checks the first byte as an ASCII letter. A Unicode letter and an
underscore do not satisfy either initial class. Schema, effect, handler,
effect-operation, record-field, type-parameter, and hole-label names keep their
existing casing behavior.

An invalid covered name reports `name.invalid_case` at the complete written
name token. The primary message names the required class. JSON details contain
`phase: "name"`, `origin: "source"`, the occurrence kind, the exact spelling,
the name class, and the required and observed initial classes.

An underscore-led token is retained as a recovery name in every covered
declaration or binding position. It produces the casing diagnostic without a
redundant missing-name parse diagnostic. The same token remains a named hole
in expression position. A standalone `_` remains a wildcard or discard only
where that form was already accepted.

A selected source with a casing error does not contribute its invalid names to
the checked artifact. `check` reports the error and exits unsuccessfully.

The executable evidence is:

- `examples/specification/check/identifier-casing-accepted/` for accepted
  declaration and binding classes;
- `examples/specification/check/identifier-casing-diagnostics/` for exact
  spans and structured JSON;
- `examples/specification/check/identifier-casing-underscore-recovery/` for
  underscore-led recovery without missing-name cascades;
- `examples/specification/check/identifier-casing-human/` for primary human
  diagnostics; and
- `examples/specification/run/identifier-casing-artifact-gate/` for the run
  command gate before backend execution.

Module names, use-site classification, alias target leaves, recovery
navigation, rename, and source-less lookup descriptors are outside this
implemented contract.
