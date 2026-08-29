---
role: specification
authority: normative
update-when: The shared CLI package-root selection, source discovery, dependency loading, manifest source gate, or command analysis pipeline changes.
---

# Shared Command Analysis

Before source discovery, `check`, `doc`, `fmt`, `metrics`, `repair`, `run`,
`test`, and `package lock` resolve the invocation directory to its filesystem
identity. Each command selects the nearest ancestor with a regular
`veln.toml`. The marker is inspected without following the marker itself. A
symbolic link, directory, or other non-regular marker does not select a root.
If no ancestor qualifies, the resolved invocation directory is an anonymous
package root.

An error while classifying a marker fails the command. The command does not
continue to a wider ancestor. After a root is selected, manifest loading reads
that root's manifest. A manifest read failure fails the command and does not
trigger fallback selection.

Relative command arguments remain relative to the invocation directory. An
explicit source or test input does not select another package root. Shared
ownership validation rejects an input outside the selected package or inside a
nested package.

The checked cases `package-root-from-subdirectory` and
`package-root-relative-input` are the executable command evidence for ancestor
selection and the invocation-relative input base. The `veln-project` selector
tests cover anonymous fallback, equivalent direct and symbolic starts,
non-regular markers, classification failure, and unreadable selected
manifests. The CLI harness checks the common command entry for all listed
commands.

`check`, `run`, `test`, and `repair` use one project analysis path for source
discovery, generated doctest sources when the command includes doctests, parse
diagnostics, parse-clean surface module loading, semantic diagnostics,
checked-core readiness, and selected-entry typed-IR readiness.

Each command keeps selection, output, execution, and write policy outside that
shared path. The focused command pages define those user-visible boundaries.
