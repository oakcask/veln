# Discussion Result: Module Metadata Location

Status: accepted-proposal

## Picked Question

- Should module metadata live in source files, a package manifest, or both?

## Decision

Use both, but give each field exactly one authoritative owner.

The first slice should have a package manifest for package-wide tooling
metadata: package identity, workspace membership, toolchain constraints,
external dependencies, build/test entry points, publishing policy, registry
metadata, and package-level documentation pointers. The concrete file name can
remain provisional, but examples may use `veln.toml` until the manifest format
is specified.

Source files should own compiler-semantic module metadata: module declarations,
imports, exports, public API boundaries, effect and contract attachments,
module-local invariants, and module documentation that must stay close to the
code it describes. The manifest may list modules for discovery or packaging,
but it must not override a source-level module declaration.

If a fact appears in both places, `veln check` should treat one place as
canonical and the other as a derived cache, hint, or error. A mismatch should be
reported as metadata drift with both source spans when possible. The first
slice should avoid duplicated canonical facts.

## Rationale

Parnas's modularity criterion is still the core reason to keep semantic module
facts close to source. A module is useful because it hides a design decision and
lets a reader understand one part of the system without reading all others. If
exports, effects, invariants, or module intent live only in a package-level
file, the repair loop forces agents to jump away from the code whose boundary
is being repaired.

DeRemer and Kron's programming-in-the-large distinction points the other way
for package composition. A package manager or build tool needs a stable,
machine-readable description of how larger units are connected. That concern is
not the same as the expression, function, and module syntax inside a source
file. Manifest-owned data is better for external dependencies, toolchain
version constraints, publishing, workspace membership, and build policy because
tools can read it without parsing every source file.

Existing ecosystems show the same split. Cargo uses `Cargo.toml` as a package
manifest containing package metadata and compilation-relevant configuration.
Go uses `go.mod` for module path, Go version, and module dependency
requirements. Python's `pyproject.toml` specification treats statically
declared project metadata as canonical for packaging tools while reserving
tool-specific configuration under a tool namespace. These are strong examples
for Veln's package-level manifest.

The Java module system shows why source-level module declarations still matter.
Project Jigsaw places `requires` and `exports` in `module-info.java`, compiles
the declaration into the artifact, and explicitly leaves version selection to
build tools. That is a useful precedent for Veln: semantic visibility and
phase fidelity belong with the language, while version solving and packaging
policy belong with tools.

Dependency-network research also argues against relying on manifest data
alone. Hejderup, Beller, Triantafyllou, and Gousios show that manifest-inferred
dependency networks can overgeneralize actual source use, and their Prazi work
combines manifests with call graphs to get a more precise picture. For Veln's
agent-facing checks, manifest metadata should provide the package graph, but
source analysis should remain the authority for actual imports, public surface,
effects, and reachable use.

## First-Slice Rule

- A Veln package may have a manifest for package identity, dependencies,
  toolchain constraints, workspace membership, package-level scripts or command
  aliases, publishing policy, registry metadata, and documentation pointers.
- Source-level module declarations own compiler-visible module facts:
  imports, exports, public API boundaries, effects, contracts, invariants, and
  module-local documentation comments or annotations.
- The manifest may enumerate source modules for packaging or discovery, but a
  manifest entry cannot change a module's source-level name, exports, effects,
  or contracts.
- `veln check` validates both sources of metadata when both are present and
  reports drift instead of silently choosing one.
- Generated metadata is allowed only when its generated status and canonical
  source are explicit. The checker should diagnose edits to generated derived
  metadata when that would create ambiguity.
- JSON diagnostics for metadata drift should include the canonical owner,
  derived location, field name, observed values, and repair hint.
- Small single-file programs should not require a manifest unless they need
  package identity, external dependencies, workspace membership, generated
  docs, or non-default tool configuration.

## Open Detail

The manifest file name, table layout, and exact module declaration syntax are
not decided here.

This decision also does not decide which module fields are required in the
first slice. It only decides the ownership rule: package/tool metadata belongs
in the manifest, compiler-semantic module metadata belongs in source, and
duplicated facts must have one declared authority.

## References

- Parnas, D. L. (1972). On the Criteria To Be Used in Decomposing Systems
  into Modules. *Communications of the ACM*, 15(12), 1053-1058.
  https://doi.org/10.1145/361598.361623
- DeRemer, F., & Kron, H. H. (1976). Programming-in-the-Large Versus
  Programming-in-the-Small. *IEEE Transactions on Software Engineering*, 2(2),
  80-86. https://doi.org/10.1109/TSE.1976.233534
- The Cargo Book contributors. (2026). *The Manifest Format*.
  https://doc.rust-lang.org/cargo/reference/manifest.html
- The Go Authors. (2026). *go.mod file reference*.
  https://go.dev/doc/modules/gomod-ref
- Python Packaging Authority. (2026). *pyproject.toml specification*.
  https://packaging.python.org/en/latest/specifications/pyproject-toml/
- Reinhold, M. (2015). *The State of the Module System*. OpenJDK Project
  Jigsaw. https://openjdk.org/projects/jigsaw/spec/sotms/2015-09-08
- Hejderup, J., Beller, M., Triantafyllou, K., & Gousios, G. (2021).
  *Prazi: From Package-based to Call-based Dependency Networks*.
  arXiv:2101.09563. https://arxiv.org/abs/2101.09563

## Consequence

Veln gets a tool-friendly package manifest without turning semantic module
facts into distant configuration. Agents can repair a module boundary by
reading local source and can repair package wiring by reading the manifest,
while `veln check` catches drift at the boundary between the two.
