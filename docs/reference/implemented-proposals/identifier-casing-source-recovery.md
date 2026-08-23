---
role: implementation-record
authority: supporting
update-when: Recovery-aware source identifier casing evidence is superseded or its current specification routes change.
---

# Recovery-Aware Source Identifier Casing

The first identifier-casing slice established exact written-name spans and
`name.invalid_case` for source ADT types, constructors, functions, and value
bindings. Invalid declarations are excluded from ordinary symbol lookup and
checked artifacts. Same-kind, same-scope invalid duplicates still report the
ordinary duplicate diagnostic. One unique compatible same-source recovery
reference can suppress a derivative unresolved-name diagnostic, but recovery
does not cross imports or public aliases.

`check` diagnoses every selected invalid covered name. `run` diagnoses an
invalid covered declaration or binding only when it is in the selected entry's
reachable closure. The reachable closure follows resolved declaration identity
for functions, constructors, type aliases, and module-qualified same-leaf names.

Current behavior is specified by
`../../specification/names-effects-full.md`,
`../../specification/commands-full.md`, and
`../../specification/diagnostics-json.md`. Primary executable evidence lives in
the `identifier-casing-*` check and run cases under
`../../../examples/specification/` and in the parser and semantic unit tests.
