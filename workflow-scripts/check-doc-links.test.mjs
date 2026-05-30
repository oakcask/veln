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
    route: "docs/proposals/agent-repair-loop-followups.md",
    role: "Candidate route",
  },
  {
    route: "docs/proposals/agent-test-selection-graph.md",
    role: "Candidate route",
  },
  {
    route: "docs/proposals/agent-module-package-docs.md",
    role: "Candidate route",
  },
  {
    route: "docs/proposals/agent-language-surface-expansion.md",
    role: "Candidate gate",
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
const TARGET_PROMPT = path.join("prompts", "TARGET.md");
const NO_TARGET_PROMPT = path.join("prompts", "NOTARGET");
const NO_TARGET_HEADING = "# No Proposal Target Selected";
const NO_TARGET_SUMMARY =
  "No suitable proposal target is selected for the next implementation session.";

test("repository documentation links resolve", () => {
  const result = validateDocsLinks(path.resolve("docs"));

  assert.deepEqual(result.errors, []);
  assert.equal(result.valid, true);
});

test("proposal routing preserves the no-target prompt route", () => {
  const prompt = readNoTargetPrompt();
  const proposalsIndex = readDocsFile("proposals/README.md");
  const implementationRoute = readDocsFile("proposals/implementation-route.md");
  const implementationRouteFull = readDocsFile(
    "proposals/implementation-route-full.md",
  );
  const docsIndex = readDocsFile("README.md");
  const navigation = readDocsFile("navigation.md");

  assert.equal(fs.existsSync(TARGET_PROMPT), false);
  assert.equal(
    fs.existsSync(path.join("docs", "proposals", "target-selection.md")),
    false,
  );
  assert.equal(fs.existsSync(path.join("docs", "reviews")), false);
  assertIncludes(prompt, "No suitable proposal target is selected");
  for (const route of noTargetPromptRoutes()) {
    assertIncludes(prompt, route);
  }
  assertIncludes(prompt, "does not yet name that class");
  assertIncludes(prompt, "needs a new short proposal page first");
  assertIncludes(
    proposalsIndex,
    "Use this page as a catalog only. Pick the proposal that matches the task, then\n" +
      "compare it with `../specification/` before changing behavior.",
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
    "Proposal catalog: [proposals/README.md](proposals/README.md).",
  );
});

test("no-target prompt content is the active proposal prompt state", () => {
  const prompt = readNoTargetPrompt();

  assert.equal(prompt, readNoTargetPrompt());
  assertIncludes(prompt, NO_TARGET_HEADING);
  assertIncludes(prompt, NO_TARGET_SUMMARY);
  assert.deepEqual(promptProposalRoutes(prompt), noTargetPromptRoutes());
});

test("no-target prompt routes stay classified as non-active targets", () => {
  const prompt = readNoTargetPrompt();
  const proposalsIndex = readDocsFile("proposals/README.md");
  const referenceFollowups = readDocsFile("proposals/reference-followups.md");
  const repairLoop = readDocsFile("proposals/agent-repair-loop-followups.md");
  const testSelection = readDocsFile("proposals/agent-test-selection-graph.md");
  const modulePackageDocs = readDocsFile(
    "proposals/agent-module-package-docs.md",
  );
  const languageSurface = readDocsFile(
    "proposals/agent-language-surface-expansion.md",
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
    "repair verification,\n  ranking, partial application, and automatic repair follow-ups beyond the\n  current command boundary",
  );

  assertProposalIndexRoutes(proposalsIndex);
  assertIncludes(
    proposalsIndex,
    "This directory catalogs planned or accepted work that is not fully documented",
  );
  assertIncludes(
    proposalsIndex,
    "## Catalog",
  );
  assertIncludes(
    proposalsIndex,
    "declarative test harness and command analysis follow-ups.",
  );
  assertIncludes(
    proposalsIndex,
    "[agent-repair-loop-followups.md](agent-repair-loop-followups.md)",
  );
  assertIncludes(
    proposalsIndex,
    "[self-hosting-standard-library.md](self-hosting-standard-library.md)",
  );
  assertIncludes(
    proposalsIndex,
    "broad follow-up inventory that should be split into narrower proposal pages",
  );

  assertIncludes(referenceFollowups, "This page is an index");
  assertIncludes(
    referenceFollowups,
    "A listed area should\nroute to one short proposal page before implementation work starts.",
  );
  assertIncludes(
    referenceFollowups,
    "Proposal catalog: [README.md](README.md#catalog).",
  );
  assertIncludes(
    referenceFollowups,
    "Keep\nconcrete candidate wording on the linked short proposal pages",
  );
  assertIncludes(
    repairLoop,
    "repair-loop work that remains beyond the implemented",
  );
  assertIncludes(
    testSelection,
    "dependency-aware test selection",
  );
  assertIncludes(
    modulePackageDocs,
    "# Agent Module, Package, And Documentation Model",
  );
  assertIncludes(
    languageSurface,
    "# Agent Language Surface Expansion",
  );
  assertIncludes(selfHosting, "Status: proposed");
  assertIncludes(
    selfHosting,
    "Choose exactly one descriptor-only pure helper before promoting future helper\n" +
      "work into a concrete target.",
  );
});

test("no-target prompt keeps candidate routes out of implementation flow", () => {
  const prompt = readNoTargetPrompt();
  const proposalsIndex = readDocsFile("proposals/README.md");
  const implementationRoute = readDocsFile("proposals/implementation-route.md");

  const promptRoutes = [...new Set(promptProposalRoutes(prompt))];
  const catalogRoutes = proposalCatalogRoutes(proposalsIndex);

  assert.deepEqual(promptRoutes, noTargetPromptRoutes());
  assert.deepEqual(
    promptRoutes
      .filter((route) => !noTargetRoutingRoutes().includes(route))
      .toSorted(),
    catalogRoutes
      .filter((route) => route !== "docs/proposals/reference-followups.md")
      .toSorted(),
  );
  assert.ok(
    promptRoutes.every((route) => !route.endsWith("-full.md")),
    "no-target prompt must route through short proposal pages",
  );
  assert.ok(
    catalogRoutes.every((route) => !route.endsWith("-full.md")),
    "proposal catalog must route short proposal pages",
  );
  assert.ok(
    promptRoutes.every((route) => !route.startsWith("docs/specification/")),
    "no-target prompt must not select implemented specification pages",
  );
  assertIncludes(
    proposalsIndex,
    "Pick the proposal that matches the task, then\n" +
      "compare it with `../specification/` before changing behavior.",
  );
  assert.equal(
    promptRoutes.includes("docs/proposals/implementation-route.md"),
    true,
    "prompt may mention implementation route only as a later useful pointer",
  );
  assertIncludes(
    proposalsIndex,
    "broad follow-up inventory that should be split into narrower proposal pages",
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

test("proposal catalog stays backed by short proposal routes", () => {
  const proposalsIndex = readDocsFile("proposals/README.md");

  const catalogRoutes = proposalCatalogRoutes(proposalsIndex);
  assert.deepEqual(
    catalogRoutes.toSorted(),
    [
      ...noTargetCandidateRoutes(),
      "docs/proposals/reference-followups.md",
    ].toSorted(),
  );
  assert.equal(
    catalogRoutes.includes("docs/proposals/reference-followups.md"),
    true,
  );
  assert.equal(
    catalogRoutes.includes("docs/proposals/implementation-route.md"),
    false,
  );
  for (const route of catalogRoutes) {
    assert.equal(
      route.endsWith("-full.md"),
      false,
      `catalog must use a short proposal route: ${route}`,
    );
    assertIncludes(
      readDocsFile(route.replace("docs/", "")),
      "Status: proposed",
    );
  }
  assertIncludes(
    proposalsIndex,
    "New proposal work is added, split, superseded, completed, or removed.",
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

  assert.equal(fs.existsSync(TARGET_PROMPT), false);
  assertIncludes(prompt, "No suitable proposal target is selected");
  assertIncludes(
    proposalsIndex,
    "Proposal text is not current\nlanguage behavior unless the matching specification page also states it.",
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

test("repair proposal route points completed targets to reference records", () => {
  const proposal = readDocsFile("proposals/agent-repair-loop-followups.md");
  const implemented = readDocsFile(
    "reference/implemented-proposals/repair-command-confirmation-override.md",
  );
  const repairCandidates = readDocsFile("specification/repair-candidates.md");

  assertIncludes(
    proposal,
    "Status: proposed",
  );
  assertIncludes(proposal, "../specification/repair-candidates.md");
  assertIncludes(proposal, "../specification/repair-json.md");
  assertIncludes(proposal, "../specification/commands.md");
  assertIncludes(
    proposal,
    "../reference/implemented-proposals/repair-command-confirmation-override.md",
  );
  assertIncludes(proposal, "## Proposed Targets");
  assertIncludes(proposal, "Verification commands beyond built-in");
  assertIncludes(proposal, "Partial application of a candidate's replacement set.");

  assertIncludes(implemented, "`--confirm CANDIDATE_ID`");
  assertIncludes(implemented, "`--override` requires `--confirm`");
  assertIncludes(implemented, "../../specification/repair-candidates.md");
  assertIncludes(implemented, "../../specification/repair-json.md");
  assertIncludes(implemented, "../../specification/commands.md");

  assertIncludes(
    repairCandidates,
    "`veln repair --apply --override --confirm CANDIDATE_ID`",
  );
  assertIncludes(
    repairCandidates,
    "../proposals/agent-repair-loop-followups.md",
  );
  assertIncludes(
    repairCandidates,
    "Do not promote partial application or broader automatic",
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
    "routes future source-backed prelude helper migrations through the\n" +
      "implemented standard symbol split",
  );
  assertIncludes(proposal, "## Read First");
  assertIncludes(proposal, "Status: proposed");
  assertIncludes(
    proposal,
    "Choose exactly one descriptor-only pure helper before promoting future helper\n" +
      "work into a concrete target.",
  );
  assertIncludes(proposal, "## Current Boundary");
  assertIncludes(proposal, "## Work Route");
  assertIncludes(
    proposal,
    "Choose exactly one helper from the specification's descriptor-only\n" +
      "   pure-helper list",
  );
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
    "descriptor-only pure helpers: `vec_fold`, `dict_remove`",
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

function proposalCatalogRoutes(proposalsIndex) {
  const catalog = sectionText(proposalsIndex, "Catalog");
  return Array.from(
    catalog.matchAll(/\[([^\]]+)\]\(([^)#]+\.md)(?:#[^)]+)?\)/g),
    (match) => {
      const route = match[2];
      assert.equal(
        route.startsWith("../") || route.includes("/"),
        false,
        `catalog route must stay in the proposal directory: ${route}`,
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
    "Use this page as a catalog only.",
  );
  assertIncludes(
    proposalsIndex,
    "## Catalog",
  );
  assertIncludes(
    proposalsIndex,
    "[agent-repair-loop-followups.md](agent-repair-loop-followups.md):",
  );
  assertIncludes(
    proposalsIndex,
    "[agent-test-selection-graph.md](agent-test-selection-graph.md):",
  );
  assertIncludes(
    proposalsIndex,
    "[agent-module-package-docs.md](agent-module-package-docs.md):",
  );
  assertIncludes(
    proposalsIndex,
    "[agent-language-surface-expansion.md](agent-language-surface-expansion.md):",
  );
  assertIncludes(
    proposalsIndex,
    "Proposal work becomes implemented and the resulting behavior is documented\n" +
      "  under `../specification/`.",
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
  if (fs.existsSync(TARGET_PROMPT)) {
    return fs.readFileSync(TARGET_PROMPT, "utf8").trim() || null;
  }

  if (fs.existsSync(NO_TARGET_PROMPT)) {
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
  if (fs.existsSync(NO_TARGET_PROMPT)) {
    return fs.readFileSync(NO_TARGET_PROMPT, "utf8");
  }

  return [
    NO_TARGET_HEADING,
    "",
    NO_TARGET_SUMMARY,
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
    "- `docs/proposals/agent-repair-loop-followups.md` routes repair verification,",
    "  ranking, partial application, and automatic repair follow-ups beyond the",
    "  current command boundary.",
    "- `docs/proposals/agent-test-selection-graph.md` needs graph evidence before",
    "  dependency-aware test selection can be an implementation target.",
    "- `docs/proposals/agent-module-package-docs.md` collects package metadata,",
    "  generated documentation, and export-model follow-ups that need narrower",
    "  proposals before implementation.",
    "- `docs/proposals/agent-language-surface-expansion.md` catalogs future surface",
    "  features; each feature needs a new short proposal page first.",
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
