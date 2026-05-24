# Discussion Result: ADR-Lite Decision Location

Status: accepted-proposal

## Picked Question

- Should ADR-lite decisions be language syntax, comments with structure, or
  docs generated from source annotations?

## Decision

ADR-lite decisions should be optional structured source documentation comments
attached to module declarations, package documentation, or public API
declarations. They are not first-slice core language syntax, and generated
decision documents are derived views, not the canonical record.

The canonical record is the structured source-adjacent comment. A docs tool may
extract those comments into an ADR index, per-module design notes, or release
documentation later. Long rationale, diagrams, meeting notes, and superseded
discussion bodies should remain in normal docs and be linked from the
structured comment rather than embedded into source.

## Rationale

Architecture-decision research supports recording rationale as a first-class
part of architectural knowledge. Tyree and Akerman argue for explicitly
documenting major architecture decisions so stakeholders can see the rationale,
alternatives, and consequences. Kruchten, Capilla, and Duenas frame this as a
decision view alongside other architecture views, and van Heesch, Avgeriou,
and Hilliard show that decision documentation has multiple stakeholder
concerns: detail, relationships, chronology, and involvement.

That evidence argues against leaving ADR-lite decisions as unstructured prose
in distant documents only. Veln is optimized for short repair loops, so an
agent fixing a module boundary should be able to discover relevant decisions
near the module or public API that the decision constrains. Parnas's
information-hiding criterion points in the same direction: modules hide design
decisions, and the reader repairing a module should not have to reconstruct
those decisions from unrelated package files.

The evidence does not justify making ADR-lite records executable language
syntax in the first slice. DeRemer and Kron's programming-in-the-large
distinction suggests that architecture records belong to the composition and
maintenance surface, not to expression-level semantics. Making ADRs part of
the core grammar would increase parser, formatter, compatibility, and error
recovery surface before Veln has demonstrated which decision fields are worth
checking.

Javadoc practice provides a useful middle ground. Source-adjacent
documentation comments can carry structured tags and generate documentation,
but long specifications can still live in separate files linked from the
comment. Knuth's literate programming shows the stronger form: one source
document can generate both program and readable documentation. Veln should
borrow the extraction principle, not the full literate-programming burden, for
ADR-lite records.

## First-Slice Rule

- ADR-lite decisions are optional and must not be required for a module to
  parse, typecheck, or run.
- When present, an ADR-lite record should be a reserved structured
  documentation comment attached to the nearest following module declaration,
  package documentation block, public type, public function, or explicit
  decision anchor.
- The minimum structured fields should be `id`, `status`, `scope`, `context`,
  `decision`, and `consequences`.
- `status` should allow at least `proposed`, `accepted`, `superseded`, and
  `rejected`; superseded records should link to the replacement decision when
  known.
- The compiler must ignore ADR-lite records for runtime semantics. Tooling may
  parse them for documentation, drift checks, and repair-routing diagnostics.
- Generated ADR documents must name the source anchor they came from and must
  be treated as derived output. Edits to generated ADR documents should not
  silently override source-adjacent records.
- Long rationale and evidence should be linked from the record instead of
  embedded into source once the comment would stop being scan-friendly.

## Open Detail

The exact comment delimiter, field spelling, link syntax, and generated docs
layout remain open.

This decision also leaves project policy open. A package profile may later
warn when public modules lack ADR-lite records for high-impact changes, but
the first-slice language should not reject code for missing decision prose.

## References

- Parnas, D. L. (1972). On the Criteria To Be Used in Decomposing Systems
  into Modules. *Communications of the ACM*, 15(12), 1053-1058.
  https://doi.org/10.1145/361598.361623
- DeRemer, F., & Kron, H. H. (1976). Programming-in-the-Large Versus
  Programming-in-the-Small. *IEEE Transactions on Software Engineering*, 2(2),
  80-86. https://doi.org/10.1109/TSE.1976.233534
- Tyree, J., & Akerman, A. (2005). Architecture Decisions: Demystifying
  Architecture. *IEEE Software*, 22(2), 19-27.
  https://dblp.org/rec/journals/software/TyreeA05
- Kruchten, P., Capilla, R., & Duenas, J. C. (2009). The Decision View's Role
  in Software Architecture Practice. *IEEE Software*, 26(2), 36-42.
  https://doi.org/10.1109/MS.2009.52
- van Heesch, U., Avgeriou, P., & Hilliard, R. (2012). A Documentation
  Framework for Architecture Decisions. *Journal of Systems and Software*,
  85(4), 795-820. https://doi.org/10.1016/j.jss.2011.10.017
- Knuth, D. E. (1984). Literate Programming. *The Computer Journal*, 27(2),
  97-111. https://doi.org/10.1093/comjnl/27.2.97
- Oracle. (n.d.). *How to Write Doc Comments for the Javadoc Tool*. Oracle
  Technical Resources.
  https://www.oracle.com/technical-resources/articles/java/javadoc-tool.html

## Consequence

Veln gets architecture-rationale breadcrumbs near the code agents repair,
without expanding the first-slice executable grammar. Documentation generation
can later provide a global decision view, while source remains the authority
for small, local ADR-lite records and normal docs remain the place for larger
discussion results.
