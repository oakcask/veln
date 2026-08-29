---
role: specification
authority: normative
update-when: The veln check command gate, source discovery behavior, dependency loading, or output contract changes.
---

# Check Command

`check` discovers source files, parses them, combines parse-clean files into one
surface module, runs semantic diagnostics for that module, and then lowers it
far enough to report checked-core executable blockers such as missing
expressions plus call and constructor arity mismatches. With `--json`, it
prints the check JSON envelope. Without `--json`, it prints human diagnostics
or `ok`.

Inputs are files or directories. If no path is provided, discovery recursively
selects owned regular `.veln` files below the supplied project root. A regular
file named `veln.toml` in a descendant directory makes that directory a nested
package root, so discovery excludes the directory and its descendants without
opening or parsing that manifest. A symbolic link or non-regular object named
`veln.toml` is not a boundary.

Discovery does not follow source or directory symbolic links. It skips `.git`
directories. A directory named `target` is an ordinary source directory and
receives the same nested-package handling as every other directory. An error
while classifying a boundary candidate fails discovery. The final discovered
file list is sorted and deduplicated.

Explicit directories are searched recursively, but every explicit file and
directory must remain owned by the supplied project root. Discovery rejects an
input outside that root, an input below a nested manifest root, a parent-path
escape, or an input that traverses a symbolic link below the root. A nested
package rejection identifies the input and nested package root. One rejected
input fails the complete discovery operation.

The checked cases `manifest-package-boundary-discovery`,
`deep-manifest-package-boundary`, `target-owned-source-directory`,
`target-nested-package-boundary`, `anonymous-outer-package-boundary`, and
`explicit-nested-package-boundary` are the executable command evidence for
recursive and explicit boundary handling.

If the selected project root contains `veln.toml`, the command reads package
and tool metadata, path dependency entries from
`[dependencies."package"]`, git, vendor, and mirror dependency metadata from
the same dependency tables, plus the implemented `[lib].exports` manifest list
after source discovery. Git dependency metadata must name a `git` remote plus
exactly one selector: `rev`, `tag`, or `branch`; `subdir` is optional
package-root metadata inside the selected source. Vendor dependency metadata
uses a string-valued `vendor` field naming an already available vendored
package directory. Mirror dependency metadata uses a string-valued `mirror`
field naming an already materialized source tree. Current dependency discovery
loads already available direct path, vendor, mirror, and git dependency roots
for source imports. A git dependency source may be a local path, a local
`file:` URL, or a non-local URL that has already been materialized under the
project cache by another operation. When `subdir` is present, the command loads
the package root below that repository-relative subdirectory. Source imports
do not clone, fetch, check out packages, resolve git revisions, update
dependency checksums, or write lockfiles. Current package export entries do not
add files to the selected set. Each export must be a
package-relative `.veln` source path, must use file-path spelling instead of
module-path spelling, must not name a `.test.veln` test companion, must derive
a valid source module path, must match a selected source file, and must not
duplicate another export for the same derived module path. `[modules]` is
rejected.

When a parse-clean source contains `use path from "package"`, the command
looks for a matching direct path, vendor, mirror, or already available git
dependency table in the current project manifest, requires the dependency root
to have a direct regular `veln.toml`, loads that dependency's discovered
`.veln` sources, checks that the dependency manifest's `[package].name` matches
the requested package identity, and requires the imported module path to be
listed by the dependency package's
`[lib].exports`. A dependency manifest export that names a `.test.veln`
companion is rejected before that path can contribute an exported module. The
external import contributes only public declarations and public aliases from
the exported dependency module to the importing source.

The checked cases `external-package-direct-manifest` and
`external-package-missing-direct-manifest` are executable command evidence for
direct dependency package roots during source analysis. The checked cases
`external-package-imports`, `external-package-vendor-mirror-imports`, and
`external-package-git-imports` are executable command evidence for direct path,
vendor, mirror, and git import success, including git `subdir` package-root
selection. The checked cases `external-package-import-boundaries`,
`external-package-vendor-mirror-boundaries`, and
`external-package-git-boundaries` are executable command evidence for the
matching export and public visibility boundaries.

Semantic diagnostics are suppressed for a file that has parse diagnostics.
Other parse-clean files in the same invocation may still produce semantic
diagnostics. Cross-file facts from parse-clean selected files, including
source-level imports and imported qualified calls, participate in the same
semantic analysis used by `run` and `test`.

