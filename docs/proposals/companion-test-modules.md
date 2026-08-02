# Companion Test Modules

Status: proposed

## Summary

Add companion modules as a source-file relationship and define
`module_name.test.veln` as the first companion kind. A test companion can
inspect the private declarations of its matching production module without
sharing that module's declaration or import scope.

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

A companion test needs a narrower privilege. It must inspect one matching
module, but it must not merge scopes with that module or gain access to other
modules' private declarations.

## Terminology

A **companion source** is a source file whose recognized suffix associates it
with one production source in the same package and directory.

A **test companion** is a companion source named `X.test.veln`. Its **target
source** is the same-directory file named `X.veln`. The target source's
path-derived module is the **target module**.

The `.test.veln` suffix is the only companion kind introduced by this
proposal. Other dotted suffixes do not acquire companion semantics.

## Proposed Source Model

`X.test.veln` is not a second source fragment of module `X`. It has a distinct,
test-only identity that source code cannot import or export by a module path.
Its declarations and written imports belong only to the test companion.

The test companion has friend access to its target module. Friend access
changes visibility lookup only. It does not merge declarations, aliases,
imports, inference state, or module initialization behavior.

The companion relationship is derived from the file path. Source syntax does
not declare or redirect the relationship.

For a target named `http` in the `net` source directory, its companion uses the
`.test.veln` suffix beside the target, and the target module path remains
`net::http`.

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

The qualified lookup may select a private or public declaration from the
target module. The permission applies to every declaration kind that ordinary
same-module lookup can select, including functions, types, constructors,
schemas, codecs, aliases, effects, and handlers.

The permission is exact and non-transitive:

- `X.test.veln` can inspect private declarations from `X.veln`.
- `X.test.veln` cannot inspect private declarations from a module imported by
  `X.veln`.
- A module imported directly by `X.test.veln` exposes only its ordinary public
  surface unless it is the target module.
- `Y.test.veln` cannot inspect private declarations from `X.veln`.
- `X_test.veln` cannot inspect private declarations from `X.veln`.

Bare names in the test companion resolve only in the companion's own scope and
through ordinary public-import rules. A bare name does not implicitly search
the target module.

Declarations in a test companion are test-only. A `pub` modifier in a test
companion is rejected because no production or test source can import the
companion module. Private target declarations cannot be re-exported or exposed
through aliases from the companion.

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

A valid test companion satisfies all of these conditions:

- Its path ends in `.test.veln`.
- Removing `.test` produces the path of an existing `.veln` target source.
- The target belongs to the same package.
- The target is a production source, not another companion source.

If no target exists, analysis reports a companion-target diagnostic at the
companion path. A chained name such as `X.test.test.veln` is rejected instead
of targeting `X.test.veln`.

`X.test.veln` and `X_test.veln` may coexist. They select different visibility
rules and do not conflict in module identity. A normal source named `test`
inside source directory `X` remains module `X::test`; it has no relationship
to `X.test.veln`.

External packages cannot import a companion source. Package export lists
reject companion paths. Distribution and standard-library bundles exclude
companion sources in the same way that they exclude other test-only sources.

## Command Behavior

| Command or context | Required behavior |
| --- | --- |
| `veln test` without targets | Discover test declarations in both `.test.veln` companions and `_test.veln` integration modules, plus existing same-file and doctest cases |
| `veln test X.veln` | Select `X.test.veln` when it exists, preserve the existing selection of `X_test.veln` when it exists, and include all selected sources in analysis |
| `veln test X.test.veln` | Select the companion and automatically include `X.veln` in analysis |
| `veln test X_test.veln` | Preserve existing ordinary-module and public-visibility behavior |
| `veln check` | Parse and analyze discovered companions in companion context so private-access errors are reported before `test` execution |
| `veln check X.test.veln` | Include and analyze `X.veln` as the target dependency |
| `veln run` | Exclude companion sources from production analysis; reject an explicitly supplied companion path as a test-only input |
| Documentation and package export | Exclude companion declarations from generated public documentation and reject companion paths as exports |
| Language server analysis | Diagnose the file in companion context and resolve qualified target-private references for navigation, completion, hover, and rename |

Selecting a production source preserves the existing same-base `_test.veln`
selection convention and adds the matching `.test.veln` companion convention.
Dependency-aware selection may select other integration tests under the
existing rules.

## Acceptance Cases

The following table is the primary acceptance model until executable cases
replace it. Each row describes an externally observable result.

| Case | Sources and operation | Expected result | Planned primary evidence |
| --- | --- | --- | --- |
| Private function access | `math.veln` defines private `increment`; `math.test.veln` writes `use math` and calls `math::increment` | Check succeeds and the test runs | Executable `test` specification case |
| Private type access | Target defines a private type and constructor used by its companion | Type and constructor resolve under ordinary same-module typing rules | Executable `test` specification case |
| Public target access | Companion calls a public target declaration | Check succeeds through the same qualified path | Executable `test` specification case |
| Missing `use` | Companion writes `math::increment` without `use math` | Name diagnostic identifies the unavailable module path | Human and JSON `check` cases |
| Bare target name | Companion calls bare `increment` with no local or public imported declaration | Unresolved-name diagnostic; target-private lookup is not implicit | Human and JSON `check` cases |
| Local shadow name | Companion defines local `increment` and target also defines private `increment` | Bare `increment` selects the local declaration; `math::increment` selects the target | Semantic analyzer unit test |
| Import isolation toward companion | Target imports `support`; companion does not | Bare and qualified `support` access is unavailable until the companion writes its own import | Human and JSON `check` cases |
| Import isolation toward target | Companion imports a name used but not imported by the target | Target analysis remains unchanged and reports the same result with or without the companion | Semantic analyzer regression test |
| Established private inference | Production call sites determine a private target helper's omitted signature; companion calls that helper | Companion observes the production-inferred signature | Semantic analyzer and executable `test` cases |
| Companion does not complete inference | A private target helper is underconstrained by production sources and constrained only by a companion call | The production inference diagnostic remains | Human and JSON `check` cases |
| Companion does not change effects | A companion call reaches an effectful private target helper | Target effect inference is unchanged; the companion test must declare the resulting effect | Semantic analyzer and executable `test` cases |
| Non-transitive access | Target imports `support`; companion attempts `support::private_helper` | Visibility diagnostic rejects the private declaration | Human and JSON `check` cases |
| Wrong companion | `other.test.veln` attempts a private access to `math` | Visibility diagnostic rejects the private declaration | Human and JSON `check` cases |
| Integration boundary | `math_test.veln` attempts the same private access | Visibility diagnostic rejects the private declaration | Executable integration-boundary case |
| Coexistence | Both `math.test.veln` and `math_test.veln` contain tests | Both tests are discovered; only the companion receives friend access | CLI selection case |
| Missing target | `orphan.test.veln` exists without `orphan.veln` | Companion-target diagnostic blocks checking and testing that source | Human and JSON `check` cases |
| Chained companion | `math.test.test.veln` is discovered | Companion-path diagnostic rejects the chained suffix | Human and JSON `check` cases |
| Public companion declaration | Companion contains `pub fn helper` | Diagnostic rejects `pub` in a test-only companion | Human and JSON `check` cases |
| Explicit target selection | Run `veln test math.veln` with matching `math.test.veln` and `math_test.veln` files | Both test files are selected and selection output distinguishes the two conventions | Human and JSON CLI cases |
| Explicit companion selection | Run `veln test math.test.veln` | Target is included automatically and companion tests run | Human and JSON CLI cases |
| Production exclusion | Run a production entry while its companion contains test declarations | No companion declaration enters production lowering or generated output | CLI run and backend inspection case |
| Explicit production input | Supply `math.test.veln` as a `run` source input | Command diagnostic rejects the test-only input before entry resolution | Human and JSON CLI cases |
| Tooling identity | Request definition and rename for `math::increment` from the companion | Tooling identifies the declaration in `math.veln` without treating both files as one scope | Language server integration case |

Planned executable cases should live under `../../examples/specification/`.
Compiler and command unit tests may supplement those cases, but they do not
replace command-visible human and JSON coverage.

## Diagnostics Contract

Companion diagnostics must distinguish these failed facts:

- the companion path has no matching target source;
- the companion path attempts to target another companion;
- a companion declaration uses `pub`;
- a qualified private declaration belongs to a module other than the target;
- a companion path appears in a package export list.

The primary message must state the specific failed fact at the relevant path or
source span. Related notes may identify the derived target path or the valid
companion target. JSON details must expose the companion path, the derived
target path when one exists, and a stable reason for the failure.

Exact diagnostic identifiers and wording are implementation choices until
executable human and JSON cases establish them.

## Compatibility

This proposal does not change the meaning or discovery of `_test.veln` files.
They remain ordinary path-derived modules and integration-test sources. They
continue to require public access to imported declarations.

Existing same-file tests and doctests retain their visibility behavior.
Existing source paths containing an otherwise invalid dot do not become valid
ordinary module paths. Only the exact `.test.veln` suffix receives the new
classification.

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

Represent the companion relationship separately from ordinary module identity.
Pass the requesting source's companion target into qualified visibility lookup.
Do not assign the target module identity to declarations or imports parsed from
the companion source.

Treat companion classification as shared project metadata so the analyzer,
test selector, command layer, language server, documentation generator, and
package validator use one path rule.

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
