import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  parseMarkdownSource,
  runCommand,
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
    identity_sets: identitySets(),
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
    identity_sets: identitySets(),
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
        span: leaf.span ?? { start: 0, end: [...roots.find((root) => root.id === leaf.id).text].length },
        reviewed_text_digest: leaf.digest,
        lifecycle: leaf.lifecycle,
        destination: destinationFor(leaf.lifecycle),
      }));
    }),
  };
  const ledgerSchema = JSON.parse(fs.readFileSync("docs/reference/agent-language-services-lifecycle/migration-ledger.schema.json", "utf8"));
  const ledgerSchemaCorpus = JSON.parse(fs.readFileSync("docs/reference/agent-language-services-lifecycle/migration-ledger.schema-corpus.json", "utf8"));
  return { sourceText: source, contract, inventory, manifest, ledgerSchema, ledgerSchemaCorpus };
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
  mixed.sourceText = mixed.sourceText.replace("Request references.", "Current lifecycle and planned lifecycle share one statement.");
  const mixedRoots = parseMarkdownSource(mixed.sourceText);
  mixed.contract.roots = mixed.contract.roots.map((root, index) => ({ ...root, digest: mixedRoots[index].digest }));
  mixed.inventory.roots[3].digest = mixedRoots[3].digest;
  mixed.inventory.roots[3].children[0] = {
    ...mixed.inventory.roots[3].children[0],
    span: { start: 2, end: 61 },
    digest: digestOf(mixedRoots[3].text, 2, 61),
  };
  mixed.inventory.roots[3].children[1] = {
    ...mixed.inventory.roots[3].children[1],
    span: { start: 64, end: 86 },
    digest: digestOf(mixedRoots[3].text, 64, 86),
  };
  mixed.inventory.roots[3].separator_spans = [
    { start: 0, end: 2 },
    { start: 61, end: 64 },
    { start: 86, end: 88 },
  ];
  mixed.manifest.leaves = mixed.inventory.roots.flatMap((root) => {
    const leaves = root.children?.length > 0 ? root.children : [root];
    return leaves.map((leaf) => ({
      id: leaf.id,
      span: leaf.span ?? { start: 0, end: [...mixedRoots.find((sourceRoot) => sourceRoot.id === leaf.id).text].length },
      reviewed_text_digest: leaf.digest,
      lifecycle: leaf.lifecycle,
      destination: destinationFor(leaf.lifecycle),
    }));
  });

  assert.match(validateArtifacts({ repoRoot: ".", artifacts: wrong }).errors.join("\n"), /align inventory lifecycle/);
  assert.match(validateArtifacts({ repoRoot: ".", artifacts: mixed }).errors.join("\n"), /mixed lifecycle statement/);
});

test("rejects missing reviewed identity sets", () => {
  const artifacts = fixtures();
  artifacts.contract.identity_sets.find((set) => set.kind === "plugin_compatibility_cell").names.pop();

  const result = validateArtifacts({ repoRoot: ".", artifacts });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /add missing identity claude-code\/x86_64-unknown-linux-gnu/);
});

test("rejects mismatched inventory identity sets", () => {
  const artifacts = fixtures();
  artifacts.inventory.identity_sets.find((set) => set.kind === "saved_reference_row").names.pop();

  const result = validateArtifacts({ repoRoot: ".", artifacts });

  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /frozen-inventory.identity_sets/);
  assert.match(result.errors.join("\n"), /byte-equivalent/);
});

test("rejects missing manifest span and reviewed text digest", () => {
  const missingSpan = fixtures();
  delete missingSpan.manifest.leaves[0].span;
  const changedDigest = fixtures();
  changedDigest.manifest.leaves[0].reviewed_text_digest = "0".repeat(64);

  assert.match(validateArtifacts({ repoRoot: ".", artifacts: missingSpan }).errors.join("\n"), /record the reviewed Unicode-scalar span/);
  assert.match(validateArtifacts({ repoRoot: ".", artifacts: changedDigest }).errors.join("\n"), /reviewed source text digest/);
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
  const looseDestination = { mappings: [{ ...validMappings[0], destination: { ...validMappings[0].destination, unexpected: true } }, ...validMappings.slice(1)] };

  assert.match(validateLedger({ ledger: missing, inventory, manifest }).join("\n"), /add one mapping/);
  assert.match(validateLedger({ ledger: duplicate, inventory, manifest }).join("\n"), /duplicate mapping/);
  assert.match(validateLedger({ ledger: wildcard, inventory, manifest }).join("\n"), /range, wildcard, or catch-all/);
  assert.match(validateLedger({ ledger: directParent, inventory, manifest }).join("\n"), /parent records that declare children cannot be mapped directly/);
  assert.match(validateLedger({ ledger: removed, inventory, manifest }).join("\n"), /removed is only for supporting explanation/);
  assert.match(validateLedger({ ledger: looseDestination, inventory, manifest }).join("\n"), /unexpected destination field/);
});

test("writer modes do not overwrite reviewed inputs", () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "als-lifecycle-"));
  fs.mkdirSync(path.join(repoRoot, "docs/proposals"), { recursive: true });
  fs.mkdirSync(path.join(repoRoot, "docs/reference/agent-language-services-lifecycle"), { recursive: true });
  fs.writeFileSync(path.join(repoRoot, "docs/proposals/agent-language-services.md"), fixtures().sourceText);
  const reviewed = {
    "source-universe.json": "contract\n",
    "frozen-inventory.json": "inventory\n",
    "lifecycle-manifest.json": "manifest\n",
    "migration-ledger.schema.json": "schema\n",
  };
  for (const [file, contents] of Object.entries(reviewed)) {
    fs.writeFileSync(path.join(repoRoot, "docs/reference/agent-language-services-lifecycle", file), contents);
  }

  const result = runCommand({ command: "write-artifacts", repoRoot });

  assert.equal(result.valid, true, result.errors.join("\n"));
  for (const [file, contents] of Object.entries(reviewed)) {
    assert.equal(fs.readFileSync(path.join(repoRoot, "docs/reference/agent-language-services-lifecycle", file), "utf8"), contents);
  }
});

test("rejects blocked frozen bootstrap before allowlisting", () => {
  assert.deepEqual(validateDiffScope({
    changedPaths: ["docs/reference/agent-language-services-lifecycle/frozen-inventory.json"],
    baseHasFrozen: false,
    headHasFrozen: true,
    prerequisitesComplete: false,
  }), [
    "diff-scope: finish the checked prerequisite records before adding frozen lifecycle artifacts; the bootstrap inventory must start from the closed prerequisite universe",
  ]);
});

test("rejects stale and stacked frozen bootstrap bases", () => {
  assert.match(validateDiffScope({
    changedPaths: ["docs/reference/agent-language-services-lifecycle/frozen-inventory.json"],
    baseHasFrozen: false,
    headHasFrozen: true,
    prerequisitesComplete: true,
    baseRef: "feature-base",
    defaultRef: "main",
    defaultHasFrozen: false,
  }).join("\n"), /default branch/);

  assert.match(validateDiffScope({
    changedPaths: ["docs/reference/agent-language-services-lifecycle/frozen-inventory.json"],
    baseHasFrozen: false,
    headHasFrozen: true,
    prerequisitesComplete: true,
    baseRef: "main",
    defaultRef: "main",
    defaultHasFrozen: true,
  }).join("\n"), /already exist/);
});

test("rejects post-bootstrap immutable and type-boundary changes", () => {
  assert.match(validateDiffScope({
    changedPaths: ["docs/reference/agent-language-services-lifecycle/source-universe.json"],
    baseHasFrozen: true,
    headHasFrozen: true,
  }).join("\n"), /immutable lifecycle review path/);

  assert.match(validateDiffScope({
    changedPaths: ["workflow-scripts/check-agent-language-services-lifecycle.mjs"],
    changedEntries: [{ status: "R100", paths: ["workflow-scripts/check-agent-language-services-lifecycle.mjs", "workflow-scripts/renamed.mjs"] }],
    baseHasFrozen: true,
    headHasFrozen: true,
  }).join("\n"), /renames and Git type changes/);

  assert.match(validateDiffScope({
    changedPaths: ["crates/veln-mcp/src/server/tests.rs"],
    changedEntries: [{ status: "T", paths: ["crates/veln-mcp/src/server/tests.rs"] }],
    baseHasFrozen: false,
    headHasFrozen: true,
    prerequisitesComplete: true,
  }).join("\n"), /renames and Git type changes/);
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

  assert.match(validateDiffScope({
    changedPaths: [
      "docs/reference/agent-language-services-lifecycle/frozen-inventory.json",
      "crates/veln-cli/tests/toolchain-case-semantics.baseline",
    ],
    baseHasFrozen: false,
    headHasFrozen: true,
    prerequisitesComplete: true,
  }).join("\n"), /toolchain-case-semantics\.baseline/);
});

function destinationFor(lifecycle) {
  if (lifecycle === "current") {
    return { kind: "current", specification: "docs/specification/mcp.md", anchor: "#mcp-workspace-projects-diagnostics-and-definitions", evidence: [{ path: "crates/veln-mcp/src/server/tests.rs", case: "lifecycle_lists_and_calls_only_implemented_tools" }] };
  }
  if (lifecycle === "completed") {
    return { kind: "completed", record: "docs/reference/implemented-proposals/agent-module-package-docs.md", anchor: "#agent-module-package-and-documentation-model" };
  }
  if (lifecycle === "planned") {
    return { kind: "planned", proposal: "docs/proposals/agent-language-services.md", anchor: "#agent-language-services" };
  }
  return { kind: "removed", reason: "duplicate", duplicate_destination: "docs/proposals/agent-language-services.md" };
}

function identitySets() {
  return [
    { kind: "evidence_gate", names: range("Q", 1, 22) },
    { kind: "saved_reference_row", names: range("saved-reference-row-", 1, 6) },
    { kind: "closed_navigation_matrix_row", names: [
      "project-owned-functions",
      "source-types-and-constructors",
      "schemas",
      "public-member-aliases",
      "function-and-handler-parameters",
      "local-let-and-pattern-bindings",
      "handler-operation-clause-bindings",
      "handler-context-parameters",
      "test-companion-private-access",
      "direct-dependency-exports",
      "standard-library-declarations",
    ] },
    { kind: "closed_topic_matrix_row", names: [
      "lexical-structure-and-grammar",
      "modules-imports-packages-exports-visibility",
      "declarations-and-aliases",
      "expressions-operators-patterns",
      "types-inference-constructors",
      "effects-and-handlers",
      "contracts",
      "schemas",
      "holes",
      "tests-doc-comments-doctests",
    ] },
    { kind: "tool_or_resource_kind", names: [
      "check_project",
      "definition",
      "references",
      "search_docs",
      "read_doc",
      "workspace_projects",
      "refresh_workspace",
      "language-reference-index",
      "language-reference-topic",
      "package-documentation-index",
      "package-documentation-module",
      "package-documentation-declaration",
      "standard-library-documentation",
      "virtual-source-file",
    ] },
    { kind: "package_document_declaration_kind", names: ["contract", "function", "handler", "module", "operation", "schema", "type"] },
    { kind: "lsp_encoding", names: ["UTF-8", "UTF-16", "UTF-32"] },
    { kind: "plugin_compatibility_cell", names: ["codex/x86_64-unknown-linux-gnu", "claude-code/x86_64-unknown-linux-gnu"] },
    { kind: "unresolved_acceptance_row", names: range("unresolved-acceptance-row-", 1, 33) },
  ];
}

function range(prefix, first, last) {
  const width = String(last).length;
  return Array.from({ length: last - first + 1 }, (_, index) => `${prefix}${String(first + index).padStart(width, "0")}`);
}

function digestOf(text, start, end) {
  return crypto.createHash("sha256").update([...text].slice(start, end).join("")).digest("hex");
}
