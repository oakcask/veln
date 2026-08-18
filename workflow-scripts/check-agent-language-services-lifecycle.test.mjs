import assert from "node:assert/strict";
import crypto from "node:crypto";
import test from "node:test";
import {
  parseMarkdownSource,
  validateArtifacts,
  validateDiffScope,
  validateLedger,
} from "./check-agent-language-services-lifecycle.mjs";

function fixtures() {
  const source = [
    "---",
    "role: proposal",
    "update-when: The proposal changes.",
    "---",
    "",
    "# Agent Language Services",
    "",
    "Implemented behavior is current.",
    "",
    "| Case | Expected result |",
    "| --- | --- |",
    "| Request references. | Return planned output. |",
    "",
    "- Planned item remains unresolved.",
    "",
    "```text",
    "veln mcp",
    "```",
    "",
  ].join("\n");
  const roots = parseMarkdownSource(source);
  const contract = {
    schema_version: 1,
    source: "docs/proposals/agent-language-services.md",
    roots: roots.map((root) => ({
      id: root.id,
      heading: root.heading,
      kind: root.kind,
      conformance: true,
      digest: root.digest,
      identities: [],
    })),
  };
  const inventory = {
    schema_version: 1,
    source: "docs/proposals/agent-language-services.md",
    roots: [
      {
        id: roots[0].id,
        heading: roots[0].heading,
        kind: roots[0].kind,
        digest: roots[0].digest,
        child_count: 0,
        lifecycle: "current",
        conformance: true,
      },
      {
        id: roots[1].id,
        heading: roots[1].heading,
        kind: roots[1].kind,
        digest: roots[1].digest,
        child_count: 2,
        children: [
          {
            id: `${roots[1].id}.01`,
            heading: roots[1].heading,
            kind: "table_cell",
            span: { start: 2, end: 6 },
            digest: digestOf(roots[1].text, 2, 6),
            lifecycle: "planned",
            conformance: true,
          },
          {
            id: `${roots[1].id}.02`,
            heading: roots[1].heading,
            kind: "table_cell",
            span: { start: 9, end: 24 },
            digest: digestOf(roots[1].text, 9, 24),
            lifecycle: "planned",
            conformance: true,
          },
        ],
        separator_spans: [
          { start: 0, end: 2 },
          { start: 6, end: 9 },
          { start: 24, end: 26 },
        ],
      },
      {
        id: roots[2].id,
        heading: roots[2].heading,
        kind: roots[2].kind,
        digest: roots[2].digest,
        child_count: 2,
        children: [
          {
            id: `${roots[2].id}.01`,
            heading: roots[2].heading,
            kind: "table_cell",
            span: { start: 2, end: 5 },
            digest: digestOf(roots[2].text, 2, 5),
            lifecycle: "planned",
            conformance: true,
          },
          {
            id: `${roots[2].id}.02`,
            heading: roots[2].heading,
            kind: "table_cell",
            span: { start: 8, end: 11 },
            digest: digestOf(roots[2].text, 8, 11),
            lifecycle: "planned",
            conformance: true,
          },
        ],
        separator_spans: [
          { start: 0, end: 2 },
          { start: 5, end: 8 },
          { start: 11, end: 13 },
        ],
      },
      {
        id: roots[3].id,
        heading: roots[3].heading,
        kind: roots[3].kind,
        digest: roots[3].digest,
        child_count: 2,
        children: [
          {
            id: `${roots[3].id}.01`,
            heading: roots[3].heading,
            kind: "table_cell",
            span: { start: 2, end: 21 },
            digest: digestOf(roots[3].text, 2, 21),
            lifecycle: "planned",
            conformance: true,
          },
          {
            id: `${roots[3].id}.02`,
            heading: roots[3].heading,
            kind: "table_cell",
            span: { start: 24, end: 46 },
            digest: digestOf(roots[3].text, 24, 46),
            lifecycle: "planned",
            conformance: true,
          },
        ],
        separator_spans: [
          { start: 0, end: 2 },
          { start: 21, end: 24 },
          { start: 46, end: 48 },
        ],
      },
      {
        id: roots[4].id,
        heading: roots[4].heading,
        kind: roots[4].kind,
        digest: roots[4].digest,
        child_count: 0,
        lifecycle: "planned",
        conformance: true,
      },
      {
        id: roots[5].id,
        heading: roots[5].heading,
        kind: roots[5].kind,
        digest: roots[5].digest,
        child_count: 0,
        lifecycle: "planned",
        conformance: true,
      },
    ],
  };
  const manifest = {
    schema_version: 1,
    source: "docs/proposals/agent-language-services.md",
    leaves: inventory.roots.flatMap((root) => {
      const leaves = root.children?.length > 0 ? root.children : [root];
      return leaves.map((leaf) => ({
        id: leaf.id,
        lifecycle: leaf.lifecycle,
        destination: destinationFor(leaf.lifecycle),
      }));
    }),
  };
  const ledgerSchema = {
    $id: "https://veln-lang.invalid/schemas/agent-language-services-migration-ledger.schema.json",
    properties: {
      mappings: {
        items: {
          additionalProperties: false,
          properties: {
            lifecycle: { enum: ["current", "completed", "planned", "removed"] },
          },
        },
      },
    },
  };
  return { sourceText: source, contract, inventory, manifest, ledgerSchema };
}

test("accepts matching artifacts", () => {
  const result = validateArtifacts({
    repoRoot: ".",
    artifacts: fixtures(),
  });

  assert.equal(result.valid, true, result.errors.join("\n"));
});

test("rejects changed digests", () => {
  const artifacts = fixtures();
  artifacts.inventory.roots[0].digest = "0".repeat(64);

  const result = validateArtifacts({ repoRoot: ".", artifacts });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /frozen source text changed/);
});

test("rejects missing and duplicate inventory items", () => {
  const missing = fixtures();
  missing.inventory.roots.pop();
  const duplicate = fixtures();
  duplicate.inventory.roots.push({ ...duplicate.inventory.roots[0] });

  assert.match(validateArtifacts({ repoRoot: ".", artifacts: missing }).errors.join("\n"), /add missing inventory item/);
  assert.match(validateArtifacts({ repoRoot: ".", artifacts: duplicate }).errors.join("\n"), /duplicate inventory source ID/);
});

test("rejects missing children", () => {
  const artifacts = fixtures();
  artifacts.inventory.roots[1].children.pop();

  const result = validateArtifacts({ repoRoot: ".", artifacts });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /set child_count to 1/);
  assert.match(result.errors.join("\n"), /cover non-whitespace source scalar/);
});

test("rejects gap, overlap, and out-of-range child spans", () => {
  const gap = fixtures();
  gap.inventory.roots[1].children[0].span.end = 5;
  const overlap = fixtures();
  overlap.inventory.roots[1].children[1].span.start = 5;
  const outOfRange = fixtures();
  outOfRange.inventory.roots[1].children[1].span.end = 200;

  assert.match(validateArtifacts({ repoRoot: ".", artifacts: gap }).errors.join("\n"), /cover non-whitespace source scalar/);
  assert.match(validateArtifacts({ repoRoot: ".", artifacts: overlap }).errors.join("\n"), /overlapping child span/);
  assert.match(validateArtifacts({ repoRoot: ".", artifacts: outOfRange }).errors.join("\n"), /within 0\.\./);
});

test("rejects wrong or mixed lifecycle decisions", () => {
  const wrong = fixtures();
  wrong.inventory.roots[0].lifecycle = "planned";
  const mixed = fixtures();
  mixed.inventory.roots[3].children[0].lifecycle = "removed";

  assert.match(validateArtifacts({ repoRoot: ".", artifacts: wrong }).errors.join("\n"), /align inventory lifecycle/);
  assert.match(validateArtifacts({ repoRoot: ".", artifacts: mixed }).errors.join("\n"), /removed is only for supporting explanation/);
});

test("rejects uncovered parent lifecycle statements", () => {
  const artifacts = fixtures();
  artifacts.inventory.roots[3].separator_spans[1] = { start: 21, end: 35 };
  artifacts.inventory.roots[3].children[1].span = { start: 35, end: 46 };

  const result = validateArtifacts({ repoRoot: ".", artifacts });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /separator_spans\[1\]: keep only whitespace or table punctuation/);
});

test("rejects invalid ledger mappings", () => {
  const { inventory, manifest } = fixtures();
  const validMappings = manifest.leaves.map((leaf) => ({
    source_id: leaf.id,
    lifecycle: leaf.lifecycle,
    destination: leaf.destination,
  }));
  const missing = { mappings: validMappings.slice(1) };
  const duplicate = { mappings: [...validMappings, validMappings[0]] };
  const wildcard = { mappings: [{ ...validMappings[0], source_id: "ALS-S0001..ALS-S9999" }, ...validMappings.slice(1)] };
  const directParent = { mappings: [{ ...validMappings[0], source_id: "ALS-S0002" }, ...validMappings.slice(1)] };
  const removed = { mappings: [{ ...validMappings[0], lifecycle: "removed", destination: destinationFor("removed") }, ...validMappings.slice(1)] };

  assert.match(validateLedger({ ledger: missing, inventory, manifest }).join("\n"), /add one mapping/);
  assert.match(validateLedger({ ledger: duplicate, inventory, manifest }).join("\n"), /duplicate mapping/);
  assert.match(validateLedger({ ledger: wildcard, inventory, manifest }).join("\n"), /range, wildcard, or catch-all/);
  assert.match(validateLedger({ ledger: directParent, inventory, manifest }).join("\n"), /parent records that declare children cannot be mapped directly/);
  assert.match(validateLedger({ ledger: removed, inventory, manifest }).join("\n"), /removed is only for supporting explanation/);
});

test("rejects bootstrap changes to protected paths", () => {
  assert.deepEqual(validateDiffScope({
    changedPaths: [
      "docs/reference/agent-language-services-lifecycle/frozen-inventory.json",
      "crates/veln-mcp/src/server/tests.rs",
    ],
    baseHasFrozen: false,
    headHasFrozen: true,
    prerequisitesComplete: true,
  }), [
    "crates/veln-mcp/src/server/tests.rs: restore this path or move the change to a later PR; changing it would mix toolchain behavior or reorganize the frozen proposal during the documentation-only inventory bootstrap",
  ]);
});

function destinationFor(lifecycle) {
  if (lifecycle === "current") {
    return { kind: "current", specification: "docs/specification/mcp.md", evidence: "crates/veln-mcp/src/server/tests.rs" };
  }
  if (lifecycle === "completed") {
    return { kind: "completed", record: "docs/reference/implemented-proposals/agent-language-services.md" };
  }
  if (lifecycle === "planned") {
    return { kind: "planned", proposal: "docs/proposals/agent-language-services.md" };
  }
  return { kind: "removed", reason: "duplicate", duplicate_destination: "docs/proposals/agent-language-services.md" };
}

function digestOf(text, start, end) {
  return crypto.createHash("sha256").update([...text].slice(start, end).join("")).digest("hex");
}
