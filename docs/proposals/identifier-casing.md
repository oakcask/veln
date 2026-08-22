---
role: proposal
update-when: The remaining Veln identifier casing scope, acceptance evidence, dependencies, or implementation boundary changes.
---

# Identifier Casing

## Summary

[Identifier Casing](../specification/identifier-casing.md) specifies the
implemented validation of source-written type, constructor, function, and
value-binding declarations. This proposal contains only the remaining casing
work.

The remaining work extends the same ASCII-initial classes to module identities
and classified uses. It also provides quarantined recovery navigation, rename
validation, and source-less registry validation.

This work remains a prerequisite for the complete definition and reference
matrix in [Agent Language Services](agent-language-services.md).

## Remaining Naming Contract

Every segment of a written or source-path-derived module identity, import path,
and import alias starts with an ASCII lowercase letter. Qualified uses receive
a class only when syntax, successful resolution, or one unique recovery link
fixes their semantic role. An unresolved or ambiguous intermediate segment is
not classified from spelling alone.

An alias kind fixes its target leaf class. A public function alias target leaf
starts with an ASCII lowercase letter. A public type alias target leaf starts
with an ASCII uppercase letter. A schema alias target remains casing-neutral.

The declaration and binding classes already specified as current behavior also
apply to recovery and rename. These remaining capabilities must consume the
same shared classification records rather than defining adapter-specific
rules.

## Module Diagnostics

A source-written invalid module segment reports `name.invalid_case` at the
complete segment token. It uses `name_class: "module"`,
`required_initial: "ascii_lowercase"`, and the structured source occurrence
fields from the current diagnostic contract.

A source-path-derived invalid segment has no source token. Its diagnostic uses
a zero-width primary span at the start of the source and contains
`origin: "source_path"`, `occurrence: "path_segment"`, `source_path`,
`source_kind`, `segment`, and the zero-based `segment_index`. Validation checks
only user-controlled origin segments. Companion, doctest, and generated
bookkeeping suffixes do not produce casing diagnostics.

An implicit import alias derived from the last import segment shares that name
occurrence. One invalid segment therefore produces at most one casing
diagnostic.

An invalid derived module remains available for local parse and declaration
diagnostics but is not registered as an importable module. It contributes no
exports, duplicate-module facts, cycles, documentation module, metrics module,
or backend reachability.

## Classified Uses And Alias Targets

Qualified paths are decomposed by semantic role. Supported decompositions
include `module::value`, `module::Type`, `Type::Constructor`, and
`module::Type::Constructor`. The implicit `prelude` alias remains a reserved
module role.

A qualified constructor pattern with a lowercase final segment reports one
constructor-class casing diagnostic. The invalid head remains a recovery
pattern. Constructor lookup, arity, and exhaustiveness diagnostics derived
only from that head are suppressed while nested bindings and the arm body are
still checked.

A wrong-cased function or type alias target leaf reports the matching casing
diagnostic. Independently provable target-kind or unresolved facts remain
diagnostics. A unique recovery symbol of the expected class suppresses only
derivative target failures and never enters the export namespace.

## Quarantined Recovery Symbols

An invalid declaration or binding may create a quarantined recovery symbol
under its exact spelling. A use links to it only when all of these facts hold:

- the spelling matches;
- the intended class is compatible with the use role;
- no valid candidate wins; and
- exactly one compatible recovery symbol is visible.

Recovery supports cascade suppression and language-service navigation. It does
not make the program valid. Recovery symbols do not create ambiguity, do not
cross imports, public aliases, companion privilege, dependency boundaries, or
the implicit prelude, and never reach lowering or a backend.

Definition, references, and prepare-rename may expose one unique recovery
link. Rename may use that link only for a class-correct repair.

## Rename Boundary

Rename validates the requested spelling against the selected symbol class
before producing edits. A class-changing request returns
`rename.invalid_case` and no edits. A request that would create a duplicate or
an already provable ambiguity returns `rename.conflict` and no edits.

LSP maps both failures to JSON-RPC invalid params. A future MCP rename operation
preserves the shared code and details. Source-path-derived module segments are
not rename targets and produce no file operations.

## Source-Less Symbols

A compiler-provided symbol that participates in source lookup carries an
explicit name class and a valid class spelling. Registry construction validates
all descriptors before publishing an immutable registry. Failure is atomic in
release and test builds and reports span-less
`toolchain.invalid_symbol_case` details containing `provider`, `name`,
`name_class`, and `required_initial`.

Compiler temporaries and bookkeeping names that cannot participate in source
lookup remain outside this contract.

## Diagnostic Ordering And Command Boundaries

Remaining overlap tests must establish deterministic order by source identity,
primary span, diagnostic priority, and diagnostic id. At one span, structural
diagnostics precede `name.invalid_case`, which precedes duplicate, ambiguity,
kind, unresolved, type, and lowering diagnostics.

Command-specific tests must preserve each consumer's existing selection and
reachability boundary. Invalid selected names never reach a backend. Unselected
dependencies, excluded documentation companions, unrelated language-service
snapshots, and unreachable run declarations follow their existing consumer
boundaries.

## Acceptance Evidence

The proposal is complete when checked evidence covers:

- written and source-path module segments for regular, companion, doctest, and
  generated sources;
- module, type, constructor, function, and binding roles in qualified uses;
- function and type alias target leaves, including independent target-kind and
  unresolved failures;
- recovery selection, cascade suppression, navigation, and isolation across
  lexical, module, companion, dependency, and prelude boundaries;
- rename success, invalid case, conflict, and atomic no-edit failures;
- atomic source-less registry validation in release-mode behavior;
- structural, reserved, duplicate, ambiguity, kind, unresolved, and casing
  overlap ordering; and
- language-service, dependency, documentation, metrics, and command selection
  matrices for the remaining capabilities.

Each observable row uses executable cases or focused tests. Current behavior
is promoted to `docs/specification/` and `examples/specification/` as each
slice lands. The proposal remains here until every row above is implemented.
