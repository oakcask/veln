---
role: proposal
update-when: Legacy compiler-known network lookup or the standard network import migration changes.
---

# Remove legacy network compatibility lookup

## Outcome

The compiler will stop recognizing legacy `net::...` spellings when source has
not imported the exported standard `net` module. Checked standard-library and
executable-example sources must first use `use net from "std"`. An explicit
standard import already selects the nominal `net::IO` boundary.

After migration, an unresolved legacy spelling will produce one diagnostic
that identifies the standard import, the `net::IO` effect, and the
`net::system()` application boundary. Compatibility is not required for this
experimental project.

## Acceptance model

| Input or event | Required observation | Intended evidence |
| --- | --- | --- |
| Source uses a legacy compiler-known `net::...` spelling after checked-source migration | The fallback does not resolve, and the diagnostic identifies the import, effect, and handler migration | check and diagnostic cases |

The current [source-less lookup specification](../specification/source-less-lookup.md)
owns the compatibility behavior until this target is complete.
