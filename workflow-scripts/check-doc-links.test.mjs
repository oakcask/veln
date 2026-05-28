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
  const docsIndex = readDocsFile("README.md");
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
  assertIncludes(
    targetSelection,
    "Current decision: no active proposal target",
  );
  assertActiveTargetExampleIsNone(targetSelection);

  assertProposalIndexRoutes(proposalsIndex);
  assertIncludes(
    implementationRoute,
    "routes implementation and promotion mechanics; it does not choose targets or",
  );
  assertIncludes(
    implementationRoute,
    "Stop for no-target, broad, exploratory, helper-pool, or implemented-record\n" +
      "  classes",
  );
  assertIncludes(
    implementationRoute,
    "leave\n" +
      "  `../specification/` unchanged unless an active target is later selected",
  );
  assertIncludes(
    implementationRoute,
    "Do not reclassify target state or repeat no-target evidence here",
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
    "records the no-target prompt state, owns\n" +
      "  candidate classification, and ends implementation prompts at routing",
  );
  assertIncludes(
    noTargetReview,
    "`../proposals/implementation-route.md` starts only after target selection\n" +
      "  classifies the work as an active target",
  );
  assertIncludes(
    noTargetReview,
    "The no-target prompt state has no proposal completion checklist to implement or\npromote",
  );
  assertIncludes(docsIndex, "Proposal target decision:");
  assertIncludes(
    docsIndex,
    "Find, confirm, or reject a proposal target:",
  );
  assertIncludes(
    docsIndex,
    "target-selection route answers the task",
  );
  assertIncludes(navigation, "Proposal target selection:");
  assertIncludes(
    navigation,
    "Proposal implementation after target selection classifies an active target:",
  );
  assertIncludes(
    navigation,
    "Target state is decided only by\n" +
      "  [proposals/target-selection.md](proposals/target-selection.md); other\n" +
      "  proposal pages provide details after that route names them",
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

  assertProposalIndexRoutes(proposalsIndex);
  assertIncludes(
    proposalsIndex,
    "Open candidate pages only after target selection names their class or next\n" +
      "   route:\n" +
      "   [reference-followups.md](reference-followups.md)",
  );
  assertIncludes(
    proposalsIndex,
    "[agent-language-spec-wall/README.md](agent-language-spec-wall/README.md)",
  );

  assertIncludes(
    targetSelection,
    "| Broad follow-up index | Split one implementable short proposal page before implementation. | [reference-followups.md](reference-followups.md). |",
  );
  assertIncludes(
    targetSelection,
    "| Exploratory inventory | Select or create one short proposal page before implementation. | [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md). |",
  );
  assertIncludes(
    targetSelection,
    "| Helper candidate pool | Choose exactly one descriptor-only pure helper",
  );
  assertTargetClassRoutes(targetSelection);
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

test("no-target prompt keeps candidate routes out of implementation flow", () => {
  const prompt = fs.readFileSync(path.join("prompts", "NOTARGET"), "utf8");
  const targetSelection = readDocsFile("proposals/target-selection.md");
  const implementationRoute = readDocsFile("proposals/implementation-route.md");

  const promptRoutes = [...new Set(promptProposalRoutes(prompt))];
  const targetRoutes = targetClassRoutes(targetSelection);

  assert.deepEqual(promptRoutes, noTargetPromptRoutes());
  assert.ok(
    promptRoutes.every((route) => !route.endsWith("-full.md")),
    "no-target prompt must route through short proposal pages",
  );
  assert.ok(
    promptRoutes.every((route) => !route.startsWith("docs/specification/")),
    "no-target prompt must not select implemented specification pages",
  );
  assert.ok(
    noTargetPromptRoutes()
      .filter((route) => !route.endsWith("README.md"))
      .every((route) => targetRoutes.has(route.replace("docs/proposals/", ""))),
    "no-target candidate routes must be classified by target selection",
  );
  assert.equal(
    targetRoutes.get("implementation-route.md"),
    "Active target",
    "implementation route must be reserved for active targets",
  );
  assert.equal(
    promptRoutes.includes("docs/proposals/implementation-route.md"),
    true,
    "prompt may mention implementation route only as a later useful pointer",
  );
  assertIncludes(
    targetSelection,
    "Stop before implementation, promotion, or specification updates",
  );
  assertIncludes(
    targetSelection,
    "Do not infer a target from broad follow-up indexes, exploratory inventories,\n" +
      "helper candidate pools, or implemented proposal records",
  );
  assertIncludes(
    implementationRoute,
    "Use this page after [target-selection.md](target-selection.md) names one active\n" +
      "short proposal whose behavior is absent from `../specification/`",
  );
});

test("no-target prompt state does not resolve to an active proposal", () => {
  const prompt = fs.readFileSync(path.join("prompts", "NOTARGET"), "utf8");
  const targetSelection = readDocsFile("proposals/target-selection.md");

  assert.deepEqual(tableRowsInSection(targetSelection, "## Prompt Evidence"), [
    "| Evidence | Decision |",
    "| `prompts/TARGET.md` is absent. | Do not infer a target. |",
    "| `prompts/NOTARGET` says no implementation target is selected from the current proposals. | Keep selection unset. |",
  ]);
  assertNoTargetSelectionOutcome(targetSelection);

  assert.equal(selectedTargetFromPromptState(), null);

  const classifiedPromptRoutes = classifyPromptRoutes(prompt, targetSelection);
  assert.deepEqual(classifiedPromptRoutes, new Map([
    ["docs/proposals/formatter-stabilization.md", "Implemented record"],
    ["docs/proposals/jvm-bytecode-backend.md", "Implemented record"],
    [
      "docs/proposals/agent-language-spec-wall/repair-command.md",
      "Implemented record",
    ],
    ["docs/proposals/reference-followups.md", "Broad follow-up index"],
    ["docs/proposals/agent-language-spec-wall/README.md", "Exploratory inventory"],
    ["docs/proposals/self-hosting-standard-library.md", "Helper candidate pool"],
    ["docs/proposals/README.md", "proposal index"],
    ["docs/proposals/implementation-route.md", "active-target route only"],
  ]));

  assert.equal(
    Array.from(classifiedPromptRoutes.values()).includes("Active target"),
    false,
  );
});

test("target prompt absence is covered by the no-target evidence", () => {
  const targetSelection = readDocsFile("proposals/target-selection.md");
  const implementationRoute = readDocsFile("proposals/implementation-route.md");
  const noTargetReview = readDocsFile(
    "reviews/no-proposal-target-completion.md",
  );

  assert.equal(fs.existsSync(path.join("prompts", "TARGET.md")), false);
  assert.equal(fs.existsSync(path.join("prompts", "NOTARGET")), true);
  assertNoTargetSelectionOutcome(targetSelection);
  assertIncludes(
    targetSelection,
    "completion result is the routing decision without code, promotion, or\n" +
      "specification changes",
  );
  assertIncludes(
    implementationRoute,
    "override a no-target decision",
  );
  assertIncludes(
    noTargetReview,
    "treat implementation prompts as complete without code, promotion,\n" +
      "or specification changes",
  );
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

function promptProposalRoutes(prompt) {
  return Array.from(
    prompt.matchAll(/`(docs\/proposals\/[^`]+\.md)`/g),
    (match) => match[1],
  );
}

function targetSelectionRouteSnippets() {
  return [
    "Status: routing",
    "## Read First",
    "routing decision itself is\n  complete for implementation prompts",
    "## Prompt Evidence",
    "no active proposal target",
    "## Selection Outcomes",
    "Use this table instead of reopening candidate pages just to decide whether work\n" +
      "can proceed",
    ...targetClassRouteRows(),
    "## Selection Check",
    "not a full detail\n   record, review, reference note, broad index, helper candidate pool, or\n   implemented proposal record",
    "If the behavior is already implemented, use the matching specification page",
    "If the behavior is broad, exploratory, or a helper candidate pool, split or\n   create one short proposal",
    "Do not infer a target from broad follow-up indexes, exploratory inventories,\nhelper candidate pools, or implemented proposal records",
    "For the no-target outcome, there is no proposal completion checklist",
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
    targetClassRouteRows().find((line) => line.startsWith("| Active target |")),
  );
}

function assertTargetClassRoutes(targetSelection) {
  const targetClassRows = tableRowsInSection(
    targetSelection,
    "## Selection Outcomes",
  );

  assert.deepEqual(targetClassRows, targetClassRouteRows());
}

function targetClassRouteRows() {
  return [
    "| No target | Stop before implementation, promotion, or specification updates; leave `../specification/` unchanged. | Stop here, or create one short proposal page before implementation work. |",
    "| Active target | Continue only when one short proposal page names one absent behavior. | [implementation-route.md](implementation-route.md). |",
    "| Implemented record | Treat as history or cleanup evidence; use the matching specification page for current behavior. | [formatter-stabilization.md](formatter-stabilization.md), [jvm-bytecode-backend.md](jvm-bytecode-backend.md), [agent-language-spec-wall/repair-command.md](agent-language-spec-wall/repair-command.md). |",
    "| Broad follow-up index | Split one implementable short proposal page before implementation. | [reference-followups.md](reference-followups.md). |",
    "| Exploratory inventory | Select or create one short proposal page before implementation. | [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md). |",
    "| Helper candidate pool | Choose exactly one descriptor-only pure helper, then create or select one short proposal page. | [self-hosting-standard-library.md](self-hosting-standard-library.md). |",
  ];
}

function targetClassRoutes(targetSelection) {
  const routes = new Map();
  const rows = tableRowsInSection(targetSelection, "## Selection Outcomes");

  for (const row of rows) {
    const [, targetClass, , routeCell] = row.split("|").map((cell) => cell.trim());
    for (const route of markdownLinkTargets(routeCell)) {
      routes.set(route, targetClass);
    }
  }
  return routes;
}

function assertNoTargetSelectionOutcome(targetSelection) {
  const rows = tableRowsInSection(targetSelection, "## Selection Outcomes");
  assert.equal(rows[0], targetClassRouteRows()[0]);
  assertIncludes(
    targetSelection,
    "Keep selection unset when the prompt state says no target is selected",
  );
}

function assertProposalIndexRoutes(proposalsIndex) {
  assertIncludes(
    proposalsIndex,
    "Need the current target decision, prompt evidence, stale-target checks, or\n" +
      "  candidate classification:",
  );
  assertIncludes(
    proposalsIndex,
    "Need implementation or promotion mechanics for an active target:\n" +
      "  [implementation-route.md](implementation-route.md). Open it only after\n" +
      "  target selection classifies one short proposal as active.",
  );
  assertIncludes(
    proposalsIndex,
    "## Proposal Target Flow",
  );
  assertIncludes(
    proposalsIndex,
    "Stop there when the outcome is no target, implemented record, broad index,\n" +
      "   exploratory inventory, or helper candidate pool",
  );
  assertIncludes(
    proposalsIndex,
    "Continue to [implementation-route.md](implementation-route.md) only for one\n" +
      "   active short proposal whose behavior is absent from `../specification/`",
  );
}

function classifyPromptRoutes(prompt, targetSelection) {
  const targetRoutes = targetClassRoutes(targetSelection);

  return new Map(
    promptProposalRoutes(prompt).map((route) => {
      const targetRoute = route.replace("docs/proposals/", "");
      if (targetRoute === "README.md") {
        return [route, "proposal index"];
      }
      if (targetRoute === "implementation-route.md") {
        return [route, "active-target route only"];
      }
      return [route, targetRoutes.get(targetRoute)];
    }),
  );
}

function selectedTargetFromPromptState() {
  const targetPrompt = path.join("prompts", "TARGET.md");
  if (fs.existsSync(targetPrompt)) {
    return fs.readFileSync(targetPrompt, "utf8").trim() || null;
  }

  const noTargetPrompt = path.join("prompts", "NOTARGET");
  if (fs.existsSync(noTargetPrompt)) {
    return null;
  }

  return null;
}

function tableRowsInSection(text, heading) {
  const lines = text.split("\n");
  const start = lines.findIndex((line) => line === heading);
  assert.notEqual(start, -1, `missing heading: ${heading}`);

  const rows = [];
  for (const line of lines.slice(start + 1)) {
    if (line.startsWith("## ")) {
      break;
    }
    if (
      line.startsWith("| ") &&
      !line.startsWith("| Class |") &&
      !line.startsWith("| --- |")
    ) {
      rows.push(line);
    }
  }
  return rows;
}

function markdownLinkTargets(text) {
  return Array.from(text.matchAll(/\[[^\]]+\]\(([^)]+)\)/g), (match) => match[1]);
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
