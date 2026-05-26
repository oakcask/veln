# Discussion Result: Transitive Effect Diagnostics

Status: implemented

Current reference behavior implements bounded structured provenance paths for
the direct-call, discovered-signature, and body-inferred helper effect inference
available in the first slice. Deeper provenance expansion beyond the bounded
default remains follow-up work.

## Picked Question

- How should transitive effects appear in diagnostics without overwhelming the
  agent or human reviewer?

## Decision

Transitive effect diagnostics should summarize by missing coarse effect label
and show a bounded provenance slice, not the full transitive call graph.

When a public function's inferred effects exceed its declaration, `veln check`
should make the public boundary the primary diagnostic span. For each missing
effect label, the diagnostic should show the shortest useful path from the
public function to one representative operation that introduced the effect,
plus counts for omitted equivalent paths or hidden frames. Human output should
default to this bounded view; JSON output should preserve the same bounded
provenance records with explicit truncation metadata.

## Rationale

Type-and-effect systems model effects as part of static program reasoning:
Lucassen and Gifford introduced polymorphic effect systems, Talpin and Jouvelot
formalized the type-and-effect discipline, and Koka shows that effects can be
inferred and exposed in function types. That supports Veln's earlier decision
to infer private direct and transitive effects while requiring explicit public
effect declarations.

The diagnostic question is different from the inference question. A complete
transitive closure is useful to the checker, but too noisy as the default repair
surface. Program slicing research argues for selecting the statements relevant
to a specific point of interest rather than displaying all surrounding program
text. The effect diagnostic's point of interest is "why does this public
function require `fs`?" or "why did `net` appear in this public API?" The answer
should therefore be a small effect-provenance slice: the boundary, the nearest
local call that carries the effect, and one leaf operation or primitive that
introduced it.

Question-centered debugging work such as Whyline reinforces the same product
shape. The tool should answer the repair question directly, then let later
commands or editor integrations expand the evidence when needed. First-slice
diagnostics should optimize for the next edit, not for exhaustively rendering
the internal analysis graph.

## First-Slice Rule

- Missing public effects produce a `kind: "effect"` diagnostic on the public
  function signature or effect declaration.
- Diagnostics are grouped by missing coarse effect label, such as `stdio`,
  `fs`, `net`, `db`, `time`, `random`, or `process`.
- For each missing label, human output shows at most one representative
  provenance path by default.
- The representative path should prefer the shortest path that includes:
  the public boundary, the closest source-level call in the public function or
  nearest visible helper, and the source-backed operation, built-in, foreign
  call, or runtime primitive that introduced the effect.
- Default provenance paths should be capped at three call edges. When the
  actual path is longer, the diagnostic reports how many intermediate frames
  were hidden.
- When multiple equivalent paths introduce the same effect, the diagnostic
  reports the representative path and an omitted-path count rather than listing
  every path.
- JSON diagnostics should include structured provenance records, including the
  effect label, path entries, `truncated`, `hidden_frame_count`, and
  `omitted_path_count`.
- If effect inference used incomplete metadata, the diagnostic should report
  confidence or uncertainty rather than presenting the representative path as
  complete.

## Prototype JSON Details

```json
{
  "id": "effect.missing_public_declaration",
  "severity": "error",
  "kind": "effect",
  "message": "Public function declares effects [] but may perform fs.",
  "span": {
    "file": "src/config.veln",
    "start": { "line": 4, "column": 1 },
    "end": { "line": 4, "column": 38 }
  },
  "details": {
    "declared": [],
    "inferred": ["fs"],
    "missing": [
      {
        "effect": "fs",
        "confidence": "complete",
        "provenance": [
          {
            "kind": "public_boundary",
            "symbol": "load_config",
            "span": {
              "file": "src/config.veln",
              "start": { "line": 4, "column": 1 },
              "end": { "line": 4, "column": 38 }
            }
          },
          {
            "kind": "call",
            "symbol": "read_config_file",
            "span": {
              "file": "src/config.veln",
              "start": { "line": 6, "column": 13 },
              "end": { "line": 6, "column": 38 }
            }
          },
          {
            "kind": "effect_source",
            "symbol": "fs.read_text",
            "span": {
              "file": "src/fs.veln",
              "start": { "line": 18, "column": 10 },
              "end": { "line": 18, "column": 33 }
            }
          }
        ],
        "truncated": false,
        "hidden_frame_count": 0,
        "omitted_path_count": 2
      }
    ]
  }
}
```

## Open Detail

The first implementation does not need a separate `veln explain effect`
command. If the bounded diagnostic proves too small in real examples, a later
tool can expand the same provenance graph without changing the first-slice
default.

Path selection tie-breakers can also remain implementation-defined at first.
A stable deterministic order is enough for golden diagnostics: prefer
source-backed spans over generated spans, then shorter paths, then lexical
order.

## References

- Lucassen, J. M., & Gifford, D. K. (1988). Polymorphic effect systems.
  POPL 1988. https://doi.org/10.1145/73560.73564
- Talpin, J.-P., & Jouvelot, P. (1994). The Type and Effect Discipline.
  *Information and Computation*. https://doi.org/10.1006/inco.1994.1046
- Leijen, D. (2014). Koka: Programming with Row Polymorphic Effect Types.
  arXiv:1406.2061. https://arxiv.org/abs/1406.2061
- Weiser, M. (1982). Programmers use slices when debugging.
  *Communications of the ACM*, 25(7), 446-452.
  https://doi.org/10.1145/358557.358577
- Ko, A. J., & Myers, B. A. (2004). Designing the whyline: A debugging
  interface for asking questions about program behavior. *CHI 2004*, 151-158.
  https://doi.org/10.1145/985692.985712

## Consequence

Veln keeps full transitive effect inference internally while exposing only the
repair-relevant slice by default. Agents get a small, structured answer for the
next edit; reviewers can still see whether the evidence was truncated or
incomplete.
