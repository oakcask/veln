---
role: specification
authority: normative
update-when: The exported portable path API, lexical validation, composition behavior, type boundary, or checked path evidence changes.
specification-coverage: usage=#usage; behavior=#lexical-and-composition-behavior; limits=#limits-and-errors
---

# Standard-library Portable Paths

The exported `std` package provides a pure `path` module for validated names
within an explicitly selected filesystem. A `path::Path` stores a
host-independent spelling without consulting a filesystem.

## Usage

Applications import the module and parse untrusted text before composing it:

```veln
use path as portable from "std"

pub fn child(input: String) -> Result<portable::Path, portable::InvalidPath>
	Ok(portable::join(portable::root(), portable::parse(input)?))
end
```

The module exports these pure operations:

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

`path::Path` is opaque outside its module. Callers create values through
`parse`, `root`, and `join`; they cannot name its private constructor.

## Lexical and composition behavior

The spelling `.` denotes the root. Every other valid spelling has one or more
non-empty Unicode components separated by `/`. Parsing preserves the complete
input spelling. It does not normalize case or Unicode, clean components, or
interpret backslash and colon as separators. `equal` is true exactly when the
preserved spellings are equal.

`parse` selects the first applicable result in this order:

| Input condition | Result |
| --- | --- |
| The input is empty. | `Err(path::InvalidPath::Empty)` |
| The input contains NUL. | `Err(path::InvalidPath::Nul)` |
| The complete input is `.`. | `Ok(path::root())` |
| The input starts with `/`. | `Err(path::InvalidPath::LeadingSeparator)` |
| The input ends with `/`. | `Err(path::InvalidPath::TrailingSeparator)` |
| The input contains an empty component. | `Err(path::InvalidPath::EmptyComponent)` |
| Any component is `.`. | `Err(path::InvalidPath::CurrentComponent)` |
| Any component is `..`. | `Err(path::InvalidPath::ParentComponent)` |
| No earlier condition applies. | `Ok` with the exact input spelling. |

`components(root())` is empty. Other component lists retain source order.
Joining with the root returns the other operand; joining two non-root paths
inserts one `/`. The root has no parent or name. A one-component path has the
root as its parent. Other parents remove the final component, and `name`
returns the final component.

## Limits and errors

A portable path is a lexical value, not a native host path. The module does not
select a filesystem, establish entry existence or identity, resolve links,
canonicalize host syntax, perform IO, clean input, or provide globbing. Its
type is distinct from both `String` and the compiler-known `Path` used by the
current `fs` and `process` boundaries.

Companion coverage in `crates/veln-stdlib/veln/path.test.veln` checks public
values, ordered failures, exact spelling, composition, and purity. The checked
portable-path cases under `examples/specification/check/` independently check
constructor visibility, type separation, and effect inference.
