---
review-when: Companion private-access, isolation, export, documentation, or language-server behavior changes.
---

# Companion Test Modules

Status: proposed

## Summary

Extend the existing `.test.veln` companion source boundary beyond the
implemented private-function and source ADT slices so a test companion can
inspect remaining private declaration kinds from its matching production module
without sharing that module's declaration or import scope.

Keep `module_name_test.veln` as an ordinary module and as the existing
integration-test convention. This proposal does not deprecate that convention.

## Motivation

Tests that exercise a private declaration currently have two options: place a
`test` declaration in the production source file or make the declaration
public. The first option mixes test bodies into production source. The second
option enlarges the package's public surface only for testing.

The existing `_test.veln` convention is useful for integration tests because
the file has an ordinary path-derived module identity and observes the target
through its public surface. It should retain that role.

A companion test needs a narrower privilege. The implemented function slice
lets it inspect private target functions. Remaining work extends that model to
other private declaration kinds without merging scopes or granting access to
other modules' private declarations.

## Terminology

A **companion source** is an implemented source-file relationship whose
recognized suffix associates it with one production source in the same package
and directory.

A **test companion** is a companion source named `X.test.veln`. Its **target
source** is the same-directory file named `X.veln`. The target source's
path-derived module is the **target module**.

The existing `.test.veln` suffix is the only companion kind. Other dotted
suffixes do not acquire companion semantics.

## Proposed Source Model

The implemented companion identity and command boundary are specified in
`../specification/source-surface.md` and `../specification/commands.md`.
Remaining work must preserve that boundary.

The test companion has friend access to its target module. Friend access
changes visibility lookup only. It does not merge declarations, aliases,
imports, inference state, or module initialization behavior.

## Name Resolution And Visibility

A test companion must write a normal `use` declaration for its target module.
It must use a qualified path to select a target declaration.

```veln
# parser.test.veln
use parser

test accepts_private_token() -> ()
	parser::parse_private_token()
end
```

The implemented function slice is specified in
`../specification/source-surface.md`, which routes to the checked companion
private-function and private source ADT examples.
Remaining proposal work applies the same exact permission model to other
declaration kinds that ordinary same-module lookup can select, including
schemas, codecs, aliases, effects, and handlers.

The permission is exact and non-transitive:

- `X.test.veln` can inspect the implemented private functions and source ADT
  types and constructors from `X.veln`. Remaining declaration kinds follow the
  same exact-target rule when they are implemented.
- `X.test.veln` cannot inspect private declarations from a module imported by
  `X.veln`.
- A module imported directly by `X.test.veln` exposes only its ordinary public
  surface unless it is the target module.
- `Y.test.veln` cannot inspect private declarations from `X.veln`.
- `X_test.veln` cannot inspect private declarations from `X.veln`.

Bare names in the test companion resolve only in the companion's own scope and
through ordinary public-import rules. A bare name does not implicitly search
the target module.

Declarations in a test companion are test-only. The implemented public
declaration boundary is specified in `../specification/source-surface.md`.
Remaining private-access work must preserve that boundary. Private target
declarations cannot be re-exported or exposed through aliases from the
companion.

## Import Isolation

Imports remain scoped to the source that writes them.

- An import written in `X.veln` is not available in `X.test.veln`.
- An import written in `X.test.veln` is not available in `X.veln`.
- Adding or removing a companion source cannot change name resolution in the
  target source.
- A target declaration and a companion-local declaration may use the same
  private name. A bare reference in the companion selects the companion-local
  declaration. An `X::name` reference selects the target declaration.

These constraints are normative because test-only imports must not make a
production source pass analysis only in test mode.

## Analysis Isolation

The target module's inferred types and effects must be the same with and
without its test companion. Calls from the companion do not contribute type or
effect constraints to target declarations.

If production sources provide enough constraints for a private target helper,
the companion can call the resulting inferred signature. If the helper remains
underconstrained without the companion, the production inference diagnostic
remains. A companion call cannot complete that inference.

The companion is checked after the target's production-analysis signatures are
established. This ordering is normative because it prevents a test source from
changing whether its target passes production analysis.

## File And Package Boundaries

The implemented package-publication boundary is specified in
`../specification/source-surface.md`, `../specification/commands.md`, and
`../specification/diagnostics-json.md`. Package export lists reject companion
paths, external packages cannot import a companion source through
`[lib].exports`, and distribution bundles exclude companion sources in the same
way that they exclude other test-only sources.

## Command Behavior

The implemented file and command boundary is specified in
`../specification/source-surface.md`, `../specification/commands.md`,
`../specification/test-json.md`, and `../specification/diagnostics-json.md`.
The remaining command work is limited to surfaces that need private-access,
generated-documentation, or language-server behavior.

## Acceptance Cases

The following table is the primary acceptance model until executable cases
replace it. Each row describes an externally observable result.

| Case | Sources and operation | Expected result | Planned primary evidence |
| --- | --- | --- | --- |
| Bare target name for remaining private declarations | Companion names a private target declaration without a local or public imported declaration | Unresolved-name diagnostic; target-private lookup is not implicit | Human and JSON `check` cases |
| Established remaining private inference | Production sources determine an omitted private target declaration boundary before companion checking | Companion observes only the production-established boundary | Semantic analyzer and executable `test` cases |
| Companion does not complete remaining private inference | A private target declaration is underconstrained by production sources and constrained only by a companion use | The production inference diagnostic remains | Human and JSON `check` cases |
| Companion does not change remaining private effects | A companion use reaches effectful private target behavior outside the implemented function-call and source ADT slices | Target effect inference is unchanged; the companion test must declare the resulting effect | Semantic analyzer and executable `test` cases |
| Non-transitive remaining private access | Target imports `support`; companion attempts to inspect a private remaining declaration from `support` | Visibility diagnostic rejects the private declaration | Human and JSON `check` cases |
| Wrong companion for remaining private access | `other.test.veln` attempts private remaining declaration access to `math` | Visibility diagnostic rejects the private declaration | Human and JSON `check` cases |
| Integration boundary for remaining private access | `math_test.veln` attempts remaining private declaration access outside the implemented function and source ADT slices | Visibility diagnostic rejects the private declaration | Executable integration-boundary case |
| Tooling identity | Request definition and rename for `math::increment` from the companion | Tooling identifies the declaration in `math.veln` without treating both files as one scope | Language server integration case |

Planned executable cases should live under `../../examples/specification/`.
Compiler and command unit tests may supplement those cases, but they do not
replace command-visible human and JSON coverage.

## Diagnostics Contract

Remaining companion diagnostics must distinguish these failed facts:

- a qualified private declaration belongs to a module other than the target.

The primary message must state the specific failed fact at the relevant path or
source span. Related notes may identify the valid companion target. JSON
details must expose the companion path and a stable reason for the failure.

Exact diagnostic identifiers and wording are implementation choices until
executable human and JSON cases establish them.

## Compatibility

The implemented `_test.veln`, same-file test, doctest, and exact `.test.veln`
classification behavior is specified in `../specification/source-surface.md`,
`../specification/commands.md`, and `../specification/test-json.md`.
Remaining work must preserve that behavior.

Adding a companion file can add test diagnostics and test cases. It cannot
change whether the target source succeeds under production analysis.

## Non-Goals

- Do not deprecate or rename `_test.veln` integration tests.
- Do not merge companion and target declaration scopes.
- Do not make target imports visible to the companion.
- Do not make companion imports visible to the target.
- Do not add general friend declarations between arbitrary modules.
- Do not grant transitive private access through the target's dependencies.
- Do not allow production code to import companion modules.
- Do not define additional companion kinds in this proposal.
- Do not require tests to inspect private declarations; public-surface tests
  remain preferred when private access is unnecessary.

## Rejected Alternative: Shared Module Identity

The companion does not receive the target module's identity. Sharing the
identity would merge declaration and import scopes during test analysis. A
test-only import or call-site constraint could then change target name
resolution or inference. It would also make duplicate names across the two
files ambiguous. Those outcomes conflict with production analysis isolation.

## Implementation Guidance

This section is not normative.

Keep the companion relationship separate from ordinary module identity.
Pass the requesting source's companion target into qualified visibility lookup.
Do not assign the target module identity to declarations or imports parsed from
the companion source.

Use the shared project companion classification for the analyzer, language
server, documentation generator, and package validator instead of duplicating
path rules.

## Planned Verification Commands

Implementation should make the following repository-relative checks pass:

```sh
bash scripts/agent-test -p veln-analysis
bash scripts/agent-test -p veln-sema
bash scripts/agent-test -p veln-test
bash scripts/agent-test -p veln-cli
```

The implementation must also run the specification cases added for the
acceptance table through the repository's normal specification harness.

## Completion Boundary

This proposal is complete only when all acceptance cases have checked evidence,
the current behavior is promoted to the matching pages under
`../specification/`, and companion examples are present under
`../../examples/specification/`.

After completion, move this document to
`../reference/implemented-proposals/` and remove it from the proposal catalog.
