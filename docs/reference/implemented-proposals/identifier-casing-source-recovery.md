---
role: implementation-record
authority: supporting
update-when: Recovery-aware source identifier casing evidence is superseded or its current specification routes change.
---

# Recovery-Aware Source Identifier Casing

The first identifier-casing slice established exact written-name spans and
`name.invalid_case` for source ADT types, constructors, functions, public type
and function alias declaration names, and value bindings. Invalid declarations
are excluded from ordinary symbol lookup and checked artifacts. Same-kind,
same-scope invalid duplicates still report the ordinary duplicate diagnostic.
One unique compatible same-source recovery reference can suppress a derivative
unresolved-name diagnostic, but recovery does not cross imports or public
aliases.

`check` diagnoses every selected invalid covered name. `run` diagnoses an
invalid covered declaration or binding only when it is in the selected entry's
reachable closure. The reachable closure follows resolved declaration identity
for functions, constructors, type aliases, module-qualified same-leaf names,
local binding precedence, and source ADT payload types.

Current behavior is specified by
`../../specification/names-effects-full.md`,
`../../specification/commands-full.md`, and
`../../specification/diagnostics-json.md`. Primary executable evidence lives in
the `identifier-casing-*` check and run cases under
`../../../examples/specification/` and in the parser and semantic unit tests.
The current evidence includes duplicate quarantine, public alias declaration
names, alias mismatch boundaries, import and public-alias quarantine,
selected-entry reachability, valid-candidate precedence over invalid same-leaf
peers, imported constructor precedence over invalid same-spelled local
functions, reachable handler clauses, transitive alias closure, same-file valid
declaration precedence over quarantined aliases, invalid public alias target
and same-file use preservation, split recovery candidate uniqueness, handler
clause binding recovery, invalid selected-entry rejection, local binding
precedence, source ADT payload closure, handler annotation reachability,
transitive handler body reachability, underscore-led recovered names, and
preserved non-casing diagnostics for unreachable duplicate constructors, type
aliases, and handlers.
