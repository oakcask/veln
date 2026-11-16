---
role: proposal
update-when: The planned portable path source API, lexical validity rules, composition behavior, or acceptance evidence changes.
---

# Portable Paths

Add a pure standard-library path value for names within an explicitly selected
filesystem. The value provides one host-independent syntax before the broader
filesystem and shared-IO design introduces providers, resources, or host path
conversion.

This proposal is ready for implementation. All evidence described below is
planned and must be added with the implementation.

## Scope And Source API

The exported `path` module defines an opaque `path::Path` type and the following
pure functions:

```veln
path::parse(text: String) -> Result<path::Path, path::InvalidPath>
path::root() -> path::Path
path::text(value: path::Path) -> String
path::equal(left: path::Path, right: path::Path) -> Bool
path::components(value: path::Path) -> List<String>
path::join(left: path::Path, right: path::Path) -> path::Path
path::parent(value: path::Path) -> Option<path::Path>
path::name(value: path::Path) -> Option<String>
```

`path::InvalidPath` exposes these alternatives so callers can handle invalid
input without parsing diagnostic text:

```veln
pub type InvalidPath
	pub Empty
	pub Nul
	pub LeadingSeparator
	pub TrailingSeparator
	pub EmptyComponent
	pub CurrentComponent
	pub ParentComponent
end
```

The `path::Path` constructor remains private to the module. Application code
cannot construct a value that bypasses `path::parse` or the invariant-preserving
operations. The existing compiler-known `Path` used by `fs` and `process`
remains a distinct type. Replacing that host-owned boundary belongs to the
[filesystem and shared byte IO proposal](filesystem-io.md), not this slice.

## Lexical Contract

A portable path is case-sensitive Unicode text with `/` as its only separator.
The complete spelling `.` denotes the root. Every other valid spelling contains
one or more non-empty components separated by one `/`.

`path::parse` applies the following decisions in order. The first matching
failure determines the returned `InvalidPath` alternative.

| Condition | Result |
| --- | --- |
| The input is empty. | `Err(path::InvalidPath::Empty)` |
| The input contains NUL. | `Err(path::InvalidPath::Nul)` |
| The complete input is `.`. | `Ok(path::root())` |
| The input starts with `/`. | `Err(path::InvalidPath::LeadingSeparator)` |
| The input ends with `/`. | `Err(path::InvalidPath::TrailingSeparator)` |
| A component is empty because the input contains `//`. | `Err(path::InvalidPath::EmptyComponent)` |
| A component is `.`. | `Err(path::InvalidPath::CurrentComponent)` |
| A component is `..`. | `Err(path::InvalidPath::ParentComponent)` |
| None of the preceding conditions applies. | `Ok` containing the exact input spelling. |

Backslash and colon are ordinary component characters. Parsing does not clean,
case-fold, normalize Unicode, resolve links, consult a filesystem, or reinterpret
host path syntax. Two paths are equal only when `path::text` returns the same
string value for both paths.

## Composition Contract

The pure operations preserve valid spelling and do not access a provider.

| Operation and input | Result |
| --- | --- |
| `components(root())` | `Nil` |
| `components` of the path returned by `parse("a/b")` | `Cons("a", Cons("b", Nil))` |
| `join(root(), value)` or `join(value, root())` | `value` |
| `join` of the paths returned by `parse("a")` and `parse("b/c")` | A path whose text is `a/b/c` |
| `parent(root())` | `None` |
| `parent` of the path returned by `parse("a")` | `Some(root())` |
| `parent` of the path returned by `parse("a/b")` | A path whose text is `a` |
| `name(root())` | `None` |
| `name` of the path returned by `parse("a/b")` | `Some("b")` |

`path::text` returns the accepted spelling exactly, including the root spelling.
`path::equal` observes the same exact-spelling identity. No operation in this
proposal establishes that a filesystem entry exists or that unequal spellings
refer to different host entries.

## Related Work And Design Choice

Go's [`io/fs` path contract](https://pkg.go.dev/io/fs#ValidPath) uses
host-independent, slash-separated, unrooted names and reserves `.` for the
filesystem root. It rejects empty components and `.` or `..` elements. That
contract supports provider-independent names, but Go accepts strings directly
at filesystem calls. Veln adopts the lexical vocabulary and adds an opaque
validated value so later filesystem APIs cannot receive invalid portable names.

Go's [`path` package](https://pkg.go.dev/path) provides lexical cleaning and
composition for slash-separated strings. Cleaning unsuitable input would erase
the distinction between a caller supplying a valid portable name and a caller
supplying `.` or `..` components. Veln therefore rejects non-canonical input
and makes all composition operate on already validated values.

The existing Veln host-owned `Path` is not reused because its current
[filesystem contract](../specification/effects.md#file-system-calls) explicitly
does not expose a layout, encoding, or normalization guarantee. Keeping the
types distinct prevents this pure slice from silently changing current `fs` or
`process` behavior.

## Acceptance And Verification

Implementation must add a source-defined exported `path` module.
Standard-library companion tests under `crates/veln-stdlib/veln/` are the
executable authority for the public API.

| Concern | Required observation | Planned evidence |
| --- | --- | --- |
| Public boundary | The named functions and types are available through the exported `std` package; the `path::Path` constructor is not accessible to callers. | Companion success cases and a focused compiler visibility rejection case. |
| Valid input | Root, single-component, multi-component, Unicode, backslash, and colon inputs retain their exact spelling. | Table-driven companion tests with independent expected strings and components. |
| Invalid input | Empty, NUL, boundary separators, repeated separators, and dot components return the specified alternative using the documented precedence. | Table-driven companion tests, including inputs that match more than one invalid condition. |
| Composition | Join, parent, name, and components produce the tabled results at root, one component, and multiple components. | Companion tests that compare returned text and component lists with independent expected values. |
| Type separation | `path::Path`, the existing host-owned `Path`, and `String` are not assignment-compatible. | Focused compiler type-boundary tests. |
| Purity | Every operation is callable without an effect declaration and does not consult host filesystem state. | Companion tests plus effect-inference coverage for public calls. |

The implementation must update the smallest current standard-library
specification and its executable examples or doctests so usage, behavior, and
limits do not remain proposal-only. Remove this page and its Ready catalog entry
when that evidence and implementation land.

## Non-goals

This proposal does not define native host paths, host conversion, filesystem
selection, entry identity, canonicalization, link resolution, subtree views,
confinement, globbing, extensions, IO operations, or migration of `fs::path`
and `process::cwd`. Those concerns remain in the broader filesystem proposal.
