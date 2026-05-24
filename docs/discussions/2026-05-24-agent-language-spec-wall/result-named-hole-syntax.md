# Discussion Result: Named Hole Syntax

Date: 2026-05-24

## Picked Question

- Is `_` enough for anonymous holes, or should named holes such as
  `_config_parser` be part of the first syntax?

## Decision

The first slice should support both anonymous `_` holes and named holes written
as `_lower_snake_name` in expression positions.

Anonymous holes are best for quick partial code. Named holes are part of the
source syntax because they give agents and humans a stable, intention-bearing
label for a missing expression without adding a new declaration form.

The name is a diagnostic and repair label, not a variable binding. Each hole
occurrence remains its own missing expression. Reusing a label does not mean
the holes must be filled with the same expression.

## Rationale

Typed holes are most useful when a repair tool can point to the exact missing
piece of code and preserve that target across formatting, nearby edits, and
diagnostic reruns. A source label such as `_config_parser` is easier to carry
through a repair loop than a generated ordinal such as "hole 3", and it lets
the author state intent before knowing the implementation.

Keeping `_` as the anonymous form preserves low-friction sketching. Requiring
every hole to be named would make small experiments noisier and would encourage
low-value labels such as `_todo`.

Treating a named hole as a label rather than a shared metavariable keeps the
first implementation small. Sharing constraints across multiple occurrences of
the same name would require clearer unification and lifetime semantics, and it
would make diagnostics harder to explain when two occurrences have incompatible
expected types.

## First-Slice Rules

- `_` is an anonymous expression hole.
- `_lower_snake_name` is a named expression hole. The label reported in
  diagnostics includes the leading underscore.
- Named holes are valid only where an expression is expected.
- A named hole does not introduce a binding and cannot be referenced by later
  expressions.
- Two named hole occurrences with the same label are still separate holes. The
  checker may emit a style hint when duplicate labels appear in the same
  function and could confuse repair targeting.
- Pattern wildcards and ignored bindings remain separate syntax questions. The
  first slice should avoid using `_name` as an ignored binding form.
- `veln fmt` preserves named hole labels exactly.

## Open Detail

The exact identifier grammar can start with ASCII `lower_snake_case` after the
leading underscore. Unicode identifiers, hyphenated labels, and generated hole
IDs can wait until the core repair loop needs them.

If later examples show that shared holes are useful, the language can add an
explicit form for that behavior instead of changing named-hole labels into
bindings retroactively.

## Consequence

Hole diagnostics can use source labels immediately while anonymous holes stay
available for fast sketches. Agents get stable repair anchors without forcing
the first checker to model holes as shared unknown values.
