---
role: proposal
update-when: The proposed Veln identifier casing classes, rejection diagnostics, migration scope, acceptance evidence, or implementation status changes.
---

# Identifier Casing

## Summary

Separate type and value names by their first ASCII letter. Type names and
algebraic data type (ADT) variant names start with an uppercase letter. Module
and value names start with a lowercase letter.

This rule makes a bare `Name(...)` a constructor form and a bare `name(...)` a
function or callable-value form. An accepted program cannot contain a
same-spelled callable binding and constructor, so call resolution does not need
a precedence rule for that collision.

This proposal is a language-semantics prerequisite for the complete definition
and reference matrix in
[Agent Language Services](agent-language-services.md). It is independent of
MCP and LSP transport behavior.

## Current Boundary

The lexer distinguishes identifier text but does not assign a type-name or
value-name class from its first letter. Type declarations, ADT variants,
functions, and local bindings currently accept the same identifier token.

Bare patterns already treat an uppercase initial as a constructor signal and a
lowercase initial as a binding signal. A lowercase ADT variant can therefore be
called as a constructor but cannot use the same bare spelling as a constructor
pattern. Existing navigation cases also contain a lowercase `byte` variant
that can collide with a same-spelled function or callable binding.

[Names And Effects](../specification/names-effects-full.md#name-resolution)
specifies current value shadowing. [Types](../specification/types-full.md)
specifies source ADTs and constructor resolution. Neither page defines the
identifier casing classes proposed here.

## Naming Contract

The first character of each declared name must match this table. `Uppercase`
means one ASCII letter in `A` through `Z`. `Lowercase` means one ASCII letter
in `a` through `z`.

| Name class | Required initial | Included declarations and bindings |
| --- | --- | --- |
| Type | Uppercase | Source ADT type declarations and public type aliases. |
| Constructor | Uppercase | Nullary and payload-carrying source ADT variants, whether public or private. |
| Module | Lowercase | Every segment of a written module identity, import path, and import alias. |
| Function | Lowercase | Function declarations, test declarations, and public function aliases. |
| Value binding | Lowercase | Function parameters, result bindings, local `let` bindings, match and destructuring bindings, handler context parameters, and operation-clause parameters. |

The rule applies at the declaration or binding. A qualified use does not make
an invalid declaration valid. Name lookup remains case-sensitive.

Identifiers outside the table keep their current casing behavior. This
proposal does not change schema names, effect names, handler names, effect
operation names, record fields, type parameters, or hole labels.

## Observable Rejections

Each invalid declaration or binding reports `name.invalid_case` at the complete
name span. The primary message identifies the failed fact and required class.

| Source state | Required result | Planned evidence |
| --- | --- | --- |
| A type declaration or public type alias starts with a lowercase letter. | Reject it with a message that the type name must start with an ASCII uppercase letter. | Rejected source-surface fixture for both declarations. |
| An ADT variant starts with a lowercase letter. | Reject it with a message that the constructor name must start with an ASCII uppercase letter. | Rejected fixture covering nullary, payload, public, and private variants. |
| A module identity, import-path segment, or import alias starts with an uppercase letter. | Reject the offending segment with a message that the module name must start with an ASCII lowercase letter. | Module and import boundary fixtures. |
| A function, test, or public function alias starts with an uppercase letter. | Reject it with a message that the function name must start with an ASCII lowercase letter. | Declaration and alias fixtures. |
| A value binding starts with an uppercase letter. | Reject it with a message that the binding name must start with an ASCII lowercase letter. | Table-driven parameter, result, `let`, pattern, and handler-binding cases. |

A declaration that violates this rule does not introduce a recoverable symbol
under another spelling. Analysis may continue for diagnostics, but lowering
does not produce an executable program while any casing diagnostic remains.

## Resolution Consequences

For sources without casing diagnostics:

| Source form | Candidate class |
| --- | --- |
| Bare `Name` or `Name(...)` with an uppercase initial. | ADT constructor only. |
| Bare `name` or `name(...)` with a lowercase initial. | Value binding, function, or other existing lowercase call target; never an ADT constructor. |
| Qualified `path::Name` with an uppercase final segment. | ADT constructor under the existing qualifier and visibility rules. |
| Qualified `path::name` with a lowercase final segment. | Existing non-constructor value or call target under the qualifier's rules. |

Callability remains a type property. A lowercase binding with a non-function
type still blocks the same-spelled lowercase function according to the current
value-shadowing rule. A local binding is still not visible in its own
initializer when the current specification resolves the initializer to an
outer binding or function. This proposal removes constructor-versus-value
collisions; it does not change value-versus-value shadowing.

Checking, lowering, definition, references, prepare-rename, and rename must use
the same name class. LSP and MCP must not add adapter-specific exceptions.

## Goals

- Make type, constructor, module, function, and value-binding names visually
  distinguishable at their declaration and use sites.
- Make bare constructor patterns consistent with constructor calls.
- Remove callable-binding-versus-constructor precedence from valid programs.
- Preserve current visibility, ambiguity, and value-shadowing rules within
  each name class.
- Promote the implemented contract to the current language specification and
  executable specification examples.

## Non-Goals

- Changing which constructors, functions, or bindings are visible.
- Changing duplicate-name rules inside one name class.
- Changing value-versus-value shadowing or initializer visibility.
- Requiring a complete CamelCase or snake_case word convention after the first
  character.
- Renaming schemas, effects, handlers, operations, fields, type parameters, or
  holes as part of this change.
- Defining MCP or LSP schemas, coordinates, project scope, or transport errors.

## Migration

Implementation must update repository-owned Veln sources and checked fixtures
in the same change. In particular, lowercase source ADT variants such as the
navigation fixture's `byte` constructor must receive uppercase constructor
names. Expected diagnostics, locations, documentation output, and navigation
fixtures must be updated with those source changes.

The change provides no compatibility alias for an invalid old spelling. A
lowercase function such as the standard `byte` helper remains a function and
does not become a constructor.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Check one accepted declaration from every name class. | The source passes casing validation and retains its existing semantic meaning. | Checked source-surface fixture plus focused parser and semantic tests. |
| Check every row of the rejection table. | Each invalid name reports `name.invalid_case` at exactly that name with the class-specific primary message. | Table-driven diagnostic tests and human-output fixtures. |
| Use uppercase constructors and lowercase bindings in bare and qualified expressions and patterns. | Expressions, lowering targets, pattern classification, and exhaustiveness agree on the name class. | Semantic and lowering tests plus checked ADT expression and pattern examples. |
| Attempt a former same-spelled callable-binding and constructor case. | One declaration is rejected by casing; no accepted source reaches a precedence decision between the two candidates. | Negative semantic fixture covering callable and non-callable local bindings. |
| Navigate accepted function, binding, type, and constructor uses. | The language service selects only the symbol class fixed by the initial letter. | Definition, reference, and rename cases in `veln-language-service`. |
| Run the repository specification suite after migration. | No repository-owned Veln source depends on an invalid old spelling. | Existing specification harness after fixture migration. |

This proposal is complete when all acceptance rows pass, all repository-owned
Veln sources follow the naming contract, and the implemented behavior is
stated under `docs/specification/` and routed to checked examples under
`examples/specification/`.
