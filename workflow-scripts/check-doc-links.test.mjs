import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { validateDocsLinks } from "./check-doc-links.mjs";

test("repository documentation links resolve", () => {
  const result = validateDocsLinks(path.resolve("docs"));

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("repository documentation only mentions prompt files in target selection", () => {
  const promptReferences = docsReferencesTo("prompts/");
  const referencedFiles = new Set(
    promptReferences.map((reference) => reference.relativePath),
  );

  assert.deepEqual([...referencedFiles], ["proposals/target-selection.md"]);
  assert.equal(promptReferences.length, 4);
  assert.equal(countReferencesTo(promptReferences, "prompts/TARGET.md"), 3);
  assert.equal(countReferencesTo(promptReferences, "prompts/NOTARGET"), 2);
});

test("proposal target selection preserves the no-target route", () => {
  const prompt = fs.readFileSync(path.join("prompts", "NOTARGET"), "utf8");
  const targetSelection = readDocsFile("proposals/target-selection.md");
  const proposalsIndex = readDocsFile("proposals/README.md");
  const implementationRoute = readDocsFile("proposals/implementation-route.md");
  const implementationRouteFull = readDocsFile(
    "proposals/implementation-route-full.md",
  );
  const noTargetReview = readDocsFile(
    "reviews/no-proposal-target-completion.md",
  );
  const navigation = readDocsFile("navigation.md");

  assert.equal(fs.existsSync(path.join("prompts", "TARGET.md")), false);
  assertIncludes(prompt, "No implementation target is selected");
  for (const route of noTargetPromptRoutes()) {
    assertIncludes(prompt, route);
  }
  assertIncludes(prompt, "not define one concrete short proposal target");
  assertIncludes(prompt, "current target is none");

  for (const snippet of targetSelectionRouteSnippets()) {
    assertIncludes(targetSelection, snippet);
  }
  assertIncludes(targetSelection, "`prompts/TARGET.md` is absent");
  assertIncludes(
    targetSelection,
    "`prompts/NOTARGET` says no implementation target is selected",
  );
  assertIncludes(targetSelection, "Result: no active proposal target");
  assertActiveTargetExampleIsNone(targetSelection);

  assertIncludes(
    proposalsIndex,
    "Current target and prompt evidence:",
  );
  assertIncludes(
    proposalsIndex,
    "Stop there when it says no target\n  is active",
  );
  assertIncludes(
    proposalsIndex,
    "Implementation route after one active short proposal is selected",
  );
  assertIncludes(proposalsIndex, "[target-selection.md](target-selection.md)");
  assertIncludes(
    proposalsIndex,
    "Missing, stale, broad, exploratory, or unset target",
  );
  assertIncludes(
    proposalsIndex,
    "Implementation after one active short target is selected",
  );
  assertIncludes(
    proposalsIndex,
    "Source-backed standard library helper selection",
  );
  assertIncludes(
    proposalsIndex,
    "Implemented proposal records:",
  );
  assertIncludes(
    implementationRoute,
    "routes implementation and promotion mechanics; it does not choose targets",
  );
  assertIncludes(
    implementationRoute,
    "Stop if selection is unset, broad, exploratory, or implemented history",
  );
  assertIncludes(
    implementationRoute,
    "The changed behavior is documented under `../specification/` only after code",
  );
  assertIncludes(
    implementationRoute,
    "Do not infer an active target from an implemented record or no-target state",
  );
  assertIncludes(
    implementationRouteFull,
    "selected proposal needs detailed comparison, gap evidence, or promotion cleanup",
  );
  assertIncludes(
    implementationRouteFull,
    "The short route remains the entry point and\nspecification-update router",
  );
  assertIncludes(
    implementationRouteFull,
    "Return to [implementation-route.md](implementation-route.md) for entry",
  );
  assertIncludes(
    implementationRouteFull,
    "Stop here if selection is unset, broad, exploratory, or implemented history",
  );
  assertIncludes(
    implementationRouteFull,
    "This page does not define current behavior",
  );
  assertIncludes(
    noTargetReview,
    "Status: evidence for no-target routing",
  );
  assertIncludes(
    noTargetReview,
    "records that no active target is selected\n  from the prompt state",
  );
  assertIncludes(
    noTargetReview,
    "`../proposals/implementation-route.md` starts only after target selection\n" +
      "  names one active short proposal page",
  );
  assertIncludes(
    noTargetReview,
    "The no-target prompt state has no proposal completion checklist to implement or\npromote",
  );
  assertIncludes(navigation, "Proposal target selection:");
  assertIncludes(
    navigation,
    "Proposal implementation after target selection names one active short target:",
  );
});

test("no-target prompt routes stay classified as non-active targets", () => {
  const prompt = fs.readFileSync(path.join("prompts", "NOTARGET"), "utf8");
  const proposalsIndex = readDocsFile("proposals/README.md");
  const targetSelection = readDocsFile("proposals/target-selection.md");
  const referenceFollowups = readDocsFile("proposals/reference-followups.md");
  const agentLanguageWall = readDocsFile(
    "proposals/agent-language-spec-wall/README.md",
  );
  const selfHosting = readDocsFile("proposals/self-hosting-standard-library.md");

  for (const route of noTargetPromptRoutes()) {
    assertIncludes(prompt, route);
  }

  assertIncludes(
    prompt,
    "does\n  not define one concrete short proposal target",
  );
  assertIncludes(
    prompt,
    "keeps exploratory design\n  wall material",
  );
  assertIncludes(
    prompt,
    "records completed helper\n  migrations and says the current target is none",
  );

  assertIncludes(
    proposalsIndex,
    "Missing, stale, broad, exploratory, or unset target:\n  [target-selection.md](target-selection.md)",
  );
  assertIncludes(
    proposalsIndex,
    "Broad follow-up ideas that need short target pages:\n  [reference-followups.md](reference-followups.md)",
  );
  assertIncludes(
    proposalsIndex,
    "Agent-language design-wall inventory:\n  [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md)",
  );

  assertIncludes(
    targetSelection,
    "| Broad follow-up index | Split one implementable short proposal page before implementation. | [reference-followups.md](reference-followups.md) |",
  );
  assertIncludes(
    targetSelection,
    "| Exploratory inventory | Select or create one short proposal page before implementation. | [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md) |",
  );
  assertIncludes(
    targetSelection,
    "| Helper candidate pool | Choose exactly one descriptor-only pure helper",
  );
  assertActiveTargetExampleIsNone(targetSelection);

  assertIncludes(referenceFollowups, "This page is an index");
  assertIncludes(referenceFollowups, "No follow-up target\nis active here");
  assertIncludes(
    agentLanguageWall,
    "No active target is selected from this directory as a whole",
  );
  assertIncludes(selfHosting, "Status: no active target");
  assertIncludes(selfHosting, "Current target: none");
});

test("repair proposal route covers the completed confirmation target", () => {
  const proposal = readDocsFile(
    "proposals/agent-language-spec-wall/repair-command.md",
  );
  const repairCandidates = readDocsFile("specification/repair-candidates.md");
  const openQuestions = readDocsFile(
    "proposals/agent-language-spec-wall/open-questions.md",
  );

  assertIncludes(
    proposal,
    "Status: confirmation and override target implemented",
  );
  assertIncludes(proposal, "## Completed Target");
  assertIncludes(
    proposal,
    "The confirmation and override protocol for `veln repair` is implemented",
  );
  assertIncludes(proposal, "`--confirm CANDIDATE_ID`");
  assertIncludes(proposal, "`--override` requires `--confirm`");
  assertIncludes(proposal, "../../specification/repair-candidates.md");
  assertIncludes(proposal, "../../specification/repair-json.md");
  assertIncludes(proposal, "../../specification/commands.md");
  assertIncludes(proposal, "../../specification/holes.md");
  assertIncludes(proposal, "../../specification/diagnostics-json.md");
  assertIncludes(proposal, "## Deferred Adjacent Work");
  assertIncludes(proposal, "Verification commands beyond the built-in");
  assertIncludes(proposal, "Partial application and general automatic repair");

  assertIncludes(
    repairCandidates,
    "`veln repair --apply --override --confirm CANDIDATE_ID`",
  );
  assertIncludes(
    repairCandidates,
    "../proposals/agent-language-spec-wall/repair-command.md",
  );
  assertIncludes(
    repairCandidates,
    "Do not promote partial application or broader automatic",
  );

  assertIncludes(
    openQuestions,
    "Implemented repair-loop confirmation and explicit override protocol",
  );
  assertIncludes(
    openQuestions,
    "Broader repair-loop ranking, verification, partial application",
  );
});

test("self-hosting proposal route starts from the implemented helper split", () => {
  const proposal = readDocsFile("proposals/self-hosting-standard-library.md");
  const fullProposal = readDocsFile(
    "proposals/self-hosting-standard-library-full.md",
  );
  const namesEffects = readDocsFile("specification/names-effects.md");
  const namesEffectsFull = readDocsFile("specification/names-effects-full.md");

  assertIncludes(
    proposal,
    "records completed prelude helper migrations and routes future\n" +
      "source-backed candidates back through the implemented standard symbol split",
  );
  assertIncludes(proposal, "## Read First");
  assertIncludes(proposal, "Current target: none");
  assertIncludes(proposal, "[target-selection.md](target-selection.md)");
  assertIncludes(proposal, "## Boundary");
  assertIncludes(proposal, "## Work Route");
  assertIncludes(
    proposal,
    "Choose exactly one helper from the descriptor-only pure-helper list",
  );
  assertIncludes(proposal, "## Completed Helpers");
  assertIncludes(proposal, "../specification/names-effects.md");
  assertIncludes(proposal, "../specification/source-surface.md");
  assertIncludes(
    proposal,
    "self-hosting-standard-library-full.md#remaining-pure-helper-candidates",
  );

  assertIncludes(
    fullProposal,
    "Remaining source-backed prelude work chooses from the descriptor-only pure",
  );
  assertIncludes(
    namesEffects,
    "Choosing the next self-hosting proposal target",
  );
  assertIncludes(namesEffectsFull, "### Source-Backed Boundary");
  assertIncludes(
    namesEffectsFull,
    "source-backed pure helpers: `vec_len`, `vec_is_empty`",
  );
  assertIncludes(
    namesEffectsFull,
    "source-backed pure helpers: `vec_len`, `vec_is_empty`, `vec_push`,\n  `vec_concat`, `vec_map`, `vec_filter`",
  );
  assertIncludes(
    namesEffectsFull,
    "source-backed pure helpers: `vec_len`, `vec_is_empty`, `vec_push`,\n" +
      "  `vec_concat`, `vec_map`, `vec_filter`, `vec_try_map`, `vec_try_map_with`",
  );
  assertIncludes(
    namesEffectsFull,
    "descriptor-only pure helpers: `vec_fold`, `dict_get`",
  );
});

test("reports missing markdown files and anchors", () => {
  using fixture = tempDocs("doc-links-broken");
  fixture.write(
    "README.md",
    [
      "# Start",
      "",
      "[missing file](missing.md)",
      "[missing anchor](target.md#missing)",
      "[external](https://example.test/missing)",
    ].join("\n"),
  );
  fixture.write("target.md", "# Present\n");

  const result = validateDocsLinks(fixture.root);

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "README.md:3: missing target: missing.md",
    "README.md:4: missing anchor: target.md#missing",
  ]);
});

test("resolves duplicate heading anchors and ignores fenced code links", () => {
  using fixture = tempDocs("doc-links-anchors");
  fixture.write(
    "README.md",
    [
      "# Start",
      "",
      "[second details](target.md#details-1)",
      "",
      "```",
      "[not a link](missing.md)",
      "```",
    ].join("\n"),
  );
  fixture.write("target.md", ["# Details", "", "# Details"].join("\n"));

  const result = validateDocsLinks(fixture.root);

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("ignores image links and inline code links", () => {
  using fixture = tempDocs("doc-links-ignored-syntax");
  fixture.write(
    "README.md",
    [
      "# Start",
      "",
      "![diagram](missing-image.md)",
      "`[not a link](missing-inline.md)`",
      "[real link](target.md)",
    ].join("\n"),
  );
  fixture.write("target.md", "# Present\n");

  const result = validateDocsLinks(fixture.root);

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("rejects links escaping the docs root", () => {
  using fixture = tempDocs("doc-links-escape");
  fixture.write(
    "README.md",
    ["# Start", "", "[outside](../outside.md)"].join("\n"),
  );

  const result = validateDocsLinks(fixture.root);

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "README.md:3: link escapes docs: ../outside.md",
  ]);
});

test("resolves percent-encoded local paths and anchors", () => {
  using fixture = tempDocs("doc-links-encoded");
  fixture.write(
    "README.md",
    ["# Start", "", "[encoded](topic%20map.md#named-values)"].join("\n"),
  );
  fixture.write("topic map.md", "# Named Values\n");

  const result = validateDocsLinks(fixture.root);

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

function noTargetPromptRoutes() {
  return [
    "docs/proposals/formatter-stabilization.md",
    "docs/proposals/jvm-bytecode-backend.md",
    "docs/proposals/agent-language-spec-wall/repair-command.md",
    "docs/proposals/reference-followups.md",
    "docs/proposals/agent-language-spec-wall/README.md",
    "docs/proposals/self-hosting-standard-library.md",
    "docs/proposals/README.md",
    "docs/proposals/implementation-route.md",
  ];
}

function targetSelectionRouteSnippets() {
  return [
    "Status: routing",
    "## Current Target",
    "no active proposal target",
    "## Candidate Classes",
    "| No target | Keep selection unset. Stop here or create one short proposal page. | Current prompt state |",
    "| Active target | Use [implementation-route.md](implementation-route.md). | None |",
    "| Implemented proposal record | Use the matching specification page for current behavior",
    "| Broad follow-up index | Split one implementable short proposal page before implementation. | [reference-followups.md](reference-followups.md) |",
    "| Exploratory inventory | Select or create one short proposal page before implementation. | [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md) |",
    "| Helper candidate pool | Choose exactly one descriptor-only pure helper",
    "not a full detail\n   record, review, reference note, broad index, helper candidate pool, or\n   implemented proposal record",
    "If the behavior is already implemented, use the matching specification page",
    "If the behavior is broad, exploratory, or a helper candidate pool, split or\n   create one short proposal",
    "## Handoff",
    "there is no proposal completion checklist to\n  promote into `../specification/`",
    "Leave current behavior unchanged and keep `../specification/` untouched",
    "The next implementation pass should first create or select one short proposal",
    "Do not open full proposal records until a short proposal page names the\n  specific detail needed",
  ];
}

function assertActiveTargetExampleIsNone(targetSelection) {
  const activeTargetRow =
    targetSelection
      .split("\n")
      .find((line) => line.startsWith("| Active target |")) ?? "";

  assert.equal(
    activeTargetRow,
    "| Active target | Use [implementation-route.md](implementation-route.md). | None |",
  );
}

function tempDocs(name) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), `${name}-`));
  return {
    root,
    write(relativePath, text) {
      const target = path.join(root, relativePath);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, text);
    },
    [Symbol.dispose]() {
      fs.rmSync(root, { force: true, recursive: true });
    },
  };
}

function readDocsFile(relativePath) {
  return fs.readFileSync(path.join("docs", relativePath), "utf8");
}

function docsReferencesTo(textFragment) {
  const docsRoot = path.resolve("docs");
  const references = [];

  for (const file of listMarkdownFiles(docsRoot)) {
    const text = fs.readFileSync(file, "utf8");
    const lines = text.split("\n");
    lines.forEach((line, index) => {
      if (line.includes(textFragment)) {
        references.push({
          line: index + 1,
          relativePath: path.relative(docsRoot, file),
          text: line.trim(),
        });
      }
    });
  }

  return references;
}

function countReferencesTo(references, textFragment) {
  return references.filter((reference) => reference.text.includes(textFragment))
    .length;
}

function assertIncludes(text, expected) {
  assert.ok(
    text.includes(expected),
    `expected documentation to include: ${expected}`,
  );
}

function listMarkdownFiles(root) {
  const files = [];
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const entryPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...listMarkdownFiles(entryPath));
    } else if (entry.isFile() && entry.name.endsWith(".md")) {
      files.push(entryPath);
    }
  }
  return files.sort();
}
