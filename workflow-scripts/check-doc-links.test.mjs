import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { validateDocsLinks } from "./check-doc-links.mjs";

const NO_TARGET_ROUTES = [
  {
    route: "docs/proposals/toolchain-test-harness-extensions.md",
    role: "Candidate gate",
  },
  {
    route: "docs/proposals/self-hosting-standard-library.md",
    role: "Helper candidate route",
  },
  {
    route: "docs/proposals/doctest-runtime-failure-expectations.md",
    role: "Candidate gate",
  },
  {
    route: "docs/proposals/path-runtime-representation.md",
    role: "Candidate gate",
  },
  {
    route: "docs/proposals/agent-language-spec-wall/repair-command.md",
    role: "Implemented record",
  },
  {
    route: "docs/proposals/README.md",
    role: "proposal index",
    routingOnly: true,
  },
  {
    route: "docs/proposals/implementation-route.md",
    role: "implementation route",
    routingOnly: true,
  },
];

test("repository documentation links resolve", () => {
  const result = validateDocsLinks(path.resolve("docs"));

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("proposal routing preserves the no-target prompt route", () => {
  const prompt = readNoTargetPrompt();
  const proposalsIndex = readDocsFile("proposals/README.md");
  const targetSelection = readDocsFile("proposals/target-selection.md");
  const implementationRoute = readDocsFile("proposals/implementation-route.md");
  const implementationRouteFull = readDocsFile(
    "proposals/implementation-route-full.md",
  );
  const docsIndex = readDocsFile("README.md");
  const navigation = readDocsFile("navigation.md");

  assert.equal(fs.existsSync(path.join("prompts", "TARGET.md")), false);
  assert.equal(
    fs.existsSync(path.join("docs", "proposals", "target-selection.md")),
    true,
  );
  assert.equal(fs.existsSync(path.join("docs", "reviews")), false);
  assertIncludes(prompt, "No suitable proposal target is selected");
  for (const route of noTargetPromptRoutes()) {
    assertIncludes(prompt, route);
  }
  assertIncludes(prompt, "does not yet name that class");
  assertIncludes(prompt, "needs a new short proposal page first");
  assertIncludes(
    targetSelection,
    "No concrete proposal target is selected when no concrete target prompt is\n" +
      "present or the prompt state says no target is selected.",
  );
  assertIncludes(
    targetSelection,
    "That state has no\n" +
      "proposal completion conditions to implement",
  );

  assertProposalIndexRoutes(proposalsIndex);
  assertIncludes(
    implementationRoute,
    "routes implementation and promotion mechanics",
  );
  assertIncludes(
    implementationRoute,
    "Start from the proposal page named by the task.",
  );
  assertIncludes(
    implementationRoute,
    "Stop when the target is implemented, closed, superseded, rejected, or already\n" +
      "  covered by `../specification/`.",
  );
  assertIncludes(
    implementationRoute,
    "Do not infer current behavior from proposal text; return to\n" +
      "  `../specification/` for implemented behavior.",
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
    "Stop here if the proposal is broad background, exploratory inventory, or\n" +
      "  implemented history.",
  );
  assertIncludes(
    docsIndex,
    "Promote proposal work into implemented behavior:",
  );
  assertIncludes(
    navigation,
    "Proposal implementation and promotion:",
  );
  assertIncludes(
    navigation,
    "[proposals/target-selection.md](proposals/target-selection.md) when no\n" +
      "  concrete target is named, then",
  );
});

test("no-target prompt routes stay classified as non-active targets", () => {
  const prompt = readNoTargetPrompt();
  const proposalsIndex = readDocsFile("proposals/README.md");
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
    "need a\n  clearer source-backed candidate choice",
  );
  assertIncludes(
    prompt,
    "does not yet name\n  that behavior",
  );
  assertIncludes(
    prompt,
    "current repair confirmation and override target as implemented",
  );

  assertProposalIndexRoutes(proposalsIndex);
  assertIncludes(
    proposalsIndex,
    "this page only routes to proposal areas.",
  );
  assertIncludes(
    proposalsIndex,
    "## Choose A Route",
  );
  assertIncludes(
    proposalsIndex,
    "Tests, doctests, command analysis, and harness work:",
  );
  assertIncludes(
    proposalsIndex,
    "[agent-language-spec-wall/README.md](agent-language-spec-wall/README.md)",
  );
  assertIncludes(
    proposalsIndex,
    "[self-hosting-standard-library.md](self-hosting-standard-library.md)",
  );
  assertIncludes(
    proposalsIndex,
    "Do not open `*-full.md` proposal records until a short proposal page names\n" +
      "  the section needed for the task.",
  );

  assertIncludes(referenceFollowups, "This page is an index");
  assertIncludes(
    referenceFollowups,
    "A listed area should\nroute to one short proposal page before implementation work starts.",
  );
  assertIncludes(
    referenceFollowups,
    "Current candidate gates: [target-selection.md](target-selection.md).",
  );
  assertIncludes(
    referenceFollowups,
    "Keep concrete candidate wording on the linked short proposal pages.",
  );
  assertIncludes(
    agentLanguageWall,
    "This directory as a whole is broad context; use or create one short proposal\n" +
      "  page for implementation work.",
  );
  assertIncludes(selfHosting, "Status: proposed");
  assertIncludes(
    selfHosting,
    "Choose one descriptor-only pure helper before promoting future helper work\n" +
      "into one concrete target.",
  );
});

test("no-target prompt keeps candidate routes out of implementation flow", () => {
  const prompt = readNoTargetPrompt();
  const proposalsIndex = readDocsFile("proposals/README.md");
  const targetSelection = readDocsFile("proposals/target-selection.md");
  const implementationRoute = readDocsFile("proposals/implementation-route.md");

  const promptRoutes = [...new Set(promptProposalRoutes(prompt))];
  const candidateGateRoutes = targetSelectionCandidateGateRoutes(targetSelection);

  assert.deepEqual(promptRoutes, noTargetPromptRoutes());
  assert.deepEqual(
    promptRoutes
      .filter((route) => !noTargetRoutingRoutes().includes(route))
      .toSorted(),
    candidateGateRoutes.toSorted(),
  );
  assert.ok(
    promptRoutes.every((route) => !route.endsWith("-full.md")),
    "no-target prompt must route through short proposal pages",
  );
  assert.ok(
    candidateGateRoutes.every((route) => !route.endsWith("-full.md")),
    "target selection must gate short proposal pages",
  );
  assert.ok(
    promptRoutes.every((route) => !route.startsWith("docs/specification/")),
    "no-target prompt must not select implemented specification pages",
  );
  assertIncludes(
    targetSelection,
    "Before changing code, choose an existing short proposal page or split one\n" +
      "narrow target out of the follow-up inventory.",
  );
  assert.equal(
    promptRoutes.includes("docs/proposals/implementation-route.md"),
    true,
    "prompt may mention implementation route only as a later useful pointer",
  );
  assertIncludes(
    proposalsIndex,
    "Concrete proposal page already named: read that page first, then compare it\n" +
      "  with `../specification/` before changing code.",
  );
  assertIncludes(
    implementationRoute,
    "Keep the implementation scope to the chosen proposal page unless that page\n" +
      "  routes to a full detail record or companion proposal.",
  );
  assertIncludes(
    implementationRoute,
    "Use this page after choosing a proposal page whose behavior is absent from\n" +
      "`../specification/`.",
  );
});

test("target selection gates stay backed by short proposal routes", () => {
  const targetSelection = readDocsFile("proposals/target-selection.md");

  const candidateGateRoutes = targetSelectionCandidateGateRoutes(targetSelection);

  assert.deepEqual(
    candidateGateRoutes.toSorted(),
    noTargetCandidateRoutes().toSorted(),
  );
  assert.equal(
    candidateGateRoutes.includes("docs/proposals/reference-followups.md"),
    false,
  );
  assert.equal(
    candidateGateRoutes.includes("docs/proposals/implementation-route.md"),
    false,
  );
  for (const route of candidateGateRoutes) {
    assert.equal(
      route.endsWith("-full.md"),
      false,
      `candidate gate must use a short proposal route: ${route}`,
    );
    assertIncludes(
      readDocsFile(route.replace("docs/", "")),
      "target-selection.md",
    );
  }
  assertIncludes(targetSelection, "## Selection Checklist");
  assertIncludes(
    targetSelection,
    "Keep `../specification/` unchanged until code and tests support the selected\n" +
      "   behavior.",
  );
});

test("no-target prompt state does not resolve to an active proposal", () => {
  const prompt = readNoTargetPrompt();

  assert.equal(selectedTargetFromPromptState(), null);

  const classifiedPromptRoutes = classifyPromptRoutes(prompt);
  assert.deepEqual(classifiedPromptRoutes, noTargetRouteClassMap());

  assert.equal(
    Array.from(classifiedPromptRoutes.values()).includes("Selected target"),
    false,
  );
});

test("target prompt absence is covered by proposal routing", () => {
  const proposalsIndex = readDocsFile("proposals/README.md");
  const implementationRoute = readDocsFile("proposals/implementation-route.md");
  const prompt = readNoTargetPrompt();

  assert.equal(fs.existsSync(path.join("prompts", "TARGET.md")), false);
  assertIncludes(prompt, "No suitable proposal target is selected");
  assertIncludes(
    proposalsIndex,
    "Proposal text is\n" +
      "not current language behavior unless `../specification/` also states it.",
  );
  assertIncludes(
    implementationRoute,
    "it does not override the current language specification",
  );
  assertIncludes(
    implementationRoute,
    "The changed behavior is documented under `../specification/` only after code\n" +
      "  and tests support it.",
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
  assertIncludes(proposal, "Status: proposed");
  assertIncludes(
    proposal,
    "Choose one descriptor-only pure helper before promoting future helper work\n" +
      "into one concrete target.",
  );
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

test("rejects references to unversioned paths", () => {
  using fixture = tempDocs("doc-links-unversioned");
  fixture.git("init");
  fixture.write("kept/file.md", "# Present\n");
  fixture.git("add", "kept/file.md");
  fixture.write(
    "README.md",
    [
      "# Start",
      "",
      "Versioned paths like `kept/file.md` are allowed.",
      "Do not cite `cache/generated.md`.",
      "Do not mention build output like `Entry.class`.",
    ].join("\n"),
  );

  const result = validateDocsLinks(fixture.root);

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "README.md:4: references unversioned path: cache/generated.md",
    "README.md:5: references unversioned path: Entry.class",
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
  return NO_TARGET_ROUTES.map(({ route }) => route);
}

function noTargetRoutingRoutes() {
  return NO_TARGET_ROUTES.filter(({ routingOnly }) => routingOnly).map(
    ({ route }) => route,
  );
}

function noTargetCandidateRoutes() {
  return NO_TARGET_ROUTES.filter(({ routingOnly }) => !routingOnly).map(
    ({ route }) => route,
  );
}

function noTargetRouteClassMap() {
  return new Map(NO_TARGET_ROUTES.map(({ route, role }) => [route, role]));
}

function promptProposalRoutes(prompt) {
  return Array.from(
    prompt.matchAll(/`(docs\/proposals\/[^`]+\.md)`/g),
    (match) => match[1],
  );
}

function targetSelectionCandidateGateRoutes(targetSelection) {
  const candidateGates = sectionText(targetSelection, "Candidate Gates");
  return Array.from(
    candidateGates.matchAll(/\[([^\]]+)\]\(([^)#]+\.md)(?:#[^)]+)?\)/g),
    (match) => {
      const route = match[2];
      assert.equal(
        route.startsWith("../"),
        false,
        `candidate gate must stay under proposals: ${route}`,
      );
      return `docs/proposals/${route}`;
    },
  );
}

function sectionText(markdown, heading) {
  const sectionStart = markdown.indexOf(`## ${heading}\n`);
  assert.notEqual(sectionStart, -1, `missing section: ${heading}`);
  const contentStart = sectionStart + `## ${heading}\n`.length;
  const nextSection = markdown.indexOf("\n## ", contentStart);
  if (nextSection === -1) {
    return markdown.slice(contentStart);
  }
  return markdown.slice(contentStart, nextSection);
}

function assertProposalIndexRoutes(proposalsIndex) {
  assertIncludes(
    proposalsIndex,
    "Implementation, promotion, or cleanup mechanics after a target is chosen:\n" +
      "  [implementation-route.md](implementation-route.md).",
  );
  assertIncludes(
    proposalsIndex,
    "## Start Here",
  );
  assertIncludes(
    proposalsIndex,
    "No concrete target named, or checking whether a target exists:\n" +
      "  [target-selection.md](target-selection.md).",
  );
  assertIncludes(
    proposalsIndex,
    "Stop when the matching specification page already states the behavior.",
  );
  assertIncludes(
    proposalsIndex,
    "Do not begin implementation from this index or from\n" +
      "  [reference-followups.md](reference-followups.md) alone.",
  );
  assertIncludes(
    proposalsIndex,
    "Keep candidate-gate wording in [target-selection.md](target-selection.md);",
  );
  assertIncludes(
    proposalsIndex,
    "Read [implementation-route.md](implementation-route.md) only after one short\n" +
      "  proposal page owns the target.",
  );
}

function classifyPromptRoutes(prompt) {
  const routeClasses = noTargetRouteClassMap();
  return new Map(
    promptProposalRoutes(prompt).map((route) => {
      return [route, routeClasses.get(route) ?? "Selected target"];
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

function tempDocs(name) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), `${name}-`));
  return {
    root,
    git(...args) {
      const result = spawnSync("git", args, { cwd: root });
      assert.equal(result.status, 0, result.stderr.toString());
    },
    writeRoot(relativePath, text) {
      const target = path.join(root, relativePath);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, text);
    },
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

function readNoTargetPrompt() {
  const noTargetPrompt = path.join("prompts", "NOTARGET");
  if (fs.existsSync(noTargetPrompt)) {
    return fs.readFileSync(noTargetPrompt, "utf8");
  }

  return [
    "# No Proposal Target Selected",
    "",
    "No suitable proposal target is selected for the next implementation session.",
    "",
    "Inspected proposal routes:",
    "",
    "- `docs/proposals/toolchain-test-harness-extensions.md` says no smaller target",
    "  is currently selected and the command help assertion slice is complete.",
    "- `docs/proposals/self-hosting-standard-library.md` requires choosing one",
    "  descriptor-only pure helper, but the remaining descriptor-only helpers need a",
    "  clearer source-backed candidate choice before this can be a concrete target.",
    "- `docs/proposals/doctest-runtime-failure-expectations.md` requires a concrete",
    "  runtime failure class with structured test JSON details and CLI coverage; the",
    "  page does not yet name that class beyond the implemented contract route.",
    "- `docs/proposals/path-runtime-representation.md` requires one observable path",
    "  behavior that host-string storage cannot express; the page does not yet name",
    "  that behavior.",
    "- `docs/proposals/agent-language-spec-wall/repair-command.md` records the",
    "  current repair confirmation and override target as implemented; deferred",
    "  adjacent work needs a new short proposal page first.",
    "- `docs/proposals/README.md` routes proposal areas.",
    "- `docs/proposals/implementation-route.md` applies only after a concrete",
    "  proposal target is selected.",
  ].join("\n");
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
