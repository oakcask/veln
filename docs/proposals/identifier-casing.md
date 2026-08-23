---
role: proposal
update-when: Remaining identifier casing classes, qualified-name roles, language-service behavior, or acceptance evidence changes.
---

# Complete Identifier Casing

## Outcome

Extend the implemented source declaration and binding casing foundation to
module identities, qualified occurrences, public aliases, source-less symbol
registries, and language-service operations.

Current type, constructor, function, and value-binding declaration behavior is
specified in
[`names-effects-full.md`](../specification/names-effects-full.md#name-resolution).
Its completed recovery foundation is recorded in
[`identifier-casing-source-recovery.md`](../reference/implemented-proposals/identifier-casing-source-recovery.md).

## Remaining Naming Contract

`Uppercase` means an ASCII letter in `A` through `Z`. `Lowercase` means an ASCII
letter in `a` through `z`.

| Name class | Required initial | Remaining covered source |
| --- | --- | --- |
| Type | Uppercase | Public type aliases, qualified type uses, and type-alias target leaves. |
| Constructor | Uppercase | Qualified constructor calls and pattern heads. |
| Module | Lowercase | Written and source-path-derived module segments, import paths, and import aliases. |
| Function | Lowercase | Public function aliases, qualified function uses, and function-alias target leaves. |
| Value binding | Lowercase | Remaining language-service classified binding occurrences. |

A wrong-cased source occurrence reports `name.invalid_case` at its exact token.
Source-path-derived module segments use a zero-width span at the start of the
source and identify the path segment in structured details. A standalone `_`
keeps its implemented wildcard, discard, or structural-error behavior.

## Qualified Resolution

Every resolved qualified occurrence retains a semantic role for each segment.
The shared resolver classifies the longest module or import-alias prefix, an
optional type qualifier, and the final member required by the source position.
Only a role fixed by syntax, successful resolution, or one unique recovery link
receives a casing diagnostic. An unresolved or ambiguous intermediate segment
is not classified from spelling alone.

Public function and type alias kinds fix the class of the alias name and target
leaf. Schema aliases remain casing-neutral. An invalid alias or recovery target
does not enter the export namespace.

## Command And Language-Service Boundary

Add selected and unselected evidence for `test`, included and excluded source
evidence for `doc`, snapshot and overlay evidence for language services, and
loaded and unloaded dependency evidence. These consumers preserve their
existing selection boundaries; identifier casing must not become a
workspace-global gate.

Definition, references, prepare-rename, and rename use the same classified name
roles as checking. A unique recovery link may support navigation. Rename may
use it only to produce a class-correct repair. Invalid symbols remain absent
from package snapshots and public indexes.

## Source-Less Symbols

Validate source-less registry names before publishing them. Diagnostics use
`origin = "registry"`, have no source span, and identify the registry and
provider in structured details. Invalid registry entries do not enter lookup,
snapshots, documentation, language services, or backends.

## Acceptance Evidence

- Module cases cover written paths, derived paths, imports, aliases,
  companions, doctests, and generated-source origin metadata.
- Qualified cases cover module, type, constructor, function, and alias-target
  segment roles, including unresolved and ambiguous prefixes.
- Command cases cover `test`, `doc`, language-service, and dependency selection
  boundaries.
- Navigation cases prove shared checking and language-service classification,
  unique recovery links, ambiguous recovery, and class-correct rename edits.
- Registry cases prove rejection before lookup and artifact publication.

## Non-Goals

- Unicode identifier classes.
- Case-insensitive lookup or automatic case conversion during checking.
- Changing schema, effect, handler, effect-operation, record-field, type
  parameter, or hole-label casing.
