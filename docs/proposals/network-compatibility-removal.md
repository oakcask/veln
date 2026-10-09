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

## Related work

### Explicit import provenance

- **Identity and locator:** [PEP 328, *Imports: Multi-Line and
  Absolute/Relative*](https://peps.python.org/pep-0328/).
- **Relevant finding:** Python's former package import lookup could not tell
  readers whether an unqualified import selected a package-local module or a
  top-level module. PEP 328 made the two routes syntactically distinct. It also
  staged that semantic change behind a future import and a later deprecation
  warning.
- **Applicability and limits:** This is primary design evidence for making
  module provenance explicit and for migrating callers before removing an
  implicit route. Python searches `sys.path` and distinguishes absolute from
  package-relative imports; Veln instead distinguishes an explicit exported
  standard module from compiler-known source-less lookup. The Python rollout
  therefore does not establish a required Veln compatibility period.
- **Veln consequence:** checked sources migrate to `use net from "std"`
  before compiler-known `net::...` lookup is removed. The proposal rejects an
  indefinite fallback because the same spelling would continue to have
  import-dependent provenance. It also rejects copying Python's staged warning
  window because this experimental project does not require compatibility and
  can migrate its checked corpus before removal.

### Actionable diagnostic structure

- **Identity and locator:** [Rust Compiler Development Guide, *Diagnostic and
  subdiagnostic
  structs*](https://rustc-dev-guide.rust-lang.org/diagnostics/diagnostic-structs.html).
- **Relevant finding:** rustc associates the main message with a primary span
  and represents labels, notes, help, and replacement suggestions as separate
  subdiagnostics. Suggestions also carry an applicability classification.
- **Applicability and limits:** This is implementation documentation for
  rustc's diagnostic model, not evidence that a particular message repairs a
  Veln program. Rust syntax, effect handling, and applicability categories do
  not transfer directly.
- **Veln consequence:** failure to resolve the removed legacy route remains one
  diagnostic whose primary message reports the unresolved spelling. Related
  notes identify the explicit standard import, the `net::IO` requirement, and
  the `net::system()` application boundary. The proposal rejects three
  independent errors and rejects a generic unresolved-name error with no
  migration context; both would make the single source failure harder to
  repair without adding evidence for correctness.

## Unsupported questions and access limits

The focused review covered the official Python import proposal, rustc's
official diagnostic design guide, the current Veln lookup specification, and
the checked Veln source boundary. Both external sources were publicly
accessible. Neither external source studies a compiler-known network name that
is enabled only when an explicit standard import is absent. The exact Veln
diagnostic wording and whether any non-repository user corpus still depends on
the fallback therefore remain unsupported by external evidence. Implementation
must derive the wording from Veln diagnostic conventions and must not claim
that migration of the checked corpus proves migration of unknown external
source.
