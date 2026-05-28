import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
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

test("proposal routing preserves the no-target prompt route", () => {
  const prompt = readNoTargetPrompt();
  const proposalsIndex = readDocsFile("proposals/README.md");
  const implementationRoute = readDocsFile("proposals/implementation-route.md");
  const implementationRouteFull = readDocsFile(
    "proposals/implementation-route-full.md",
  );
  const docsIndex = readDocsFile("README.md");
  const navigation = readDocsFile("navigation.md");

  assert.equal(fs.existsSync(path.join("prompts", "TARGET.md")), false);
  assert.equal(
    fs.existsSync(path.join("docs", "proposals", "target-selection.md")),
    false,
  );
  assert.equal(fs.existsSync(path.join("docs", "reviews")), false);
  assertIncludes(prompt, "No implementation target is selected");
  for (const route of noTargetPromptRoutes()) {
    assertIncludes(prompt, route);
  }
  assertIncludes(prompt, "not define one concrete short proposal target");
  assertIncludes(prompt, "current target is none");

  assertProposalIndexRoutes(proposalsIndex);
  assertIncludes(
    implementationRoute,
    "routes implementation and promotion mechanics",
  );
  assertIncludes(
    implementationRoute,
    "Start from the proposal page named by the task or from\n" +
      "  [README.md](README.md).",
  );
  assertIncludes(
    implementationRoute,
    "Stop when the proposal page is implemented, closed, superseded, rejected, or\n" +
      "  already covered by `../specification/`.",
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
    "Proposal pages are all available implementation routes, but the\n" +
      "  `specification/` pages still decide current behavior.",
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
    "Use these routes when the task names a proposal area:\n" +
      "   [reference-followups.md](reference-followups.md),",
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
    "All proposal pages are\n" +
      "   available work routes unless their own status says they are implemented,\n" +
      "   closed, superseded, or rejected.",
  );

  assertIncludes(referenceFollowups, "This page is an index");
  assertIncludes(
    referenceFollowups,
    "A listed area should\nroute to one short proposal page before implementation work starts.",
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
      "  into one concrete target.",
  );
});

test("no-target prompt keeps candidate routes out of implementation flow", () => {
  const prompt = readNoTargetPrompt();
  const proposalsIndex = readDocsFile("proposals/README.md");
  const implementationRoute = readDocsFile("proposals/implementation-route.md");

  const promptRoutes = [...new Set(promptProposalRoutes(prompt))];

  assert.deepEqual(promptRoutes, noTargetPromptRoutes());
  assert.ok(
    promptRoutes.every((route) => !route.endsWith("-full.md")),
    "no-target prompt must route through short proposal pages",
  );
  assert.ok(
    promptRoutes.every((route) => !route.startsWith("docs/specification/")),
    "no-target prompt must not select implemented specification pages",
  );
  assert.equal(
    promptRoutes.includes("docs/proposals/implementation-route.md"),
    true,
    "prompt may mention implementation route only as a later useful pointer",
  );
  assertIncludes(
    proposalsIndex,
    "Compare the proposal with `../specification/` before changing code. Stop\n" +
      "   when the specification already states the behavior.",
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

test("no-target prompt state does not resolve to an active proposal", () => {
  const prompt = readNoTargetPrompt();

  assert.equal(selectedTargetFromPromptState(), null);

  const classifiedPromptRoutes = classifyPromptRoutes(prompt);
  assert.deepEqual(classifiedPromptRoutes, new Map([
    ["docs/proposals/formatter-stabilization.md", "Implemented record"],
    ["docs/proposals/jvm-bytecode-backend.md", "Implemented record"],
    [
      "docs/proposals/agent-language-spec-wall/repair-command.md",
      "Implemented record",
    ],
    ["docs/proposals/reference-followups.md", "Broad follow-up index"],
    ["docs/proposals/agent-language-spec-wall/README.md", "Exploratory inventory"],
    ["docs/proposals/self-hosting-standard-library.md", "Helper candidate route"],
    ["docs/proposals/README.md", "proposal index"],
    ["docs/proposals/implementation-route.md", "implementation route"],
  ]));

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
  assertIncludes(prompt, "No implementation target is selected");
  assertIncludes(
    proposalsIndex,
    "Proposal text is not current language behavior unless `../specification/` also\n" +
      "states it.",
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
      "  into one concrete target.",
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

function assertProposalIndexRoutes(proposalsIndex) {
  assertIncludes(
    proposalsIndex,
    "Need implementation or promotion mechanics for proposal work:\n" +
      "  [implementation-route.md](implementation-route.md).",
  );
  assertIncludes(
    proposalsIndex,
    "## Proposal Flow",
  );
  assertIncludes(
    proposalsIndex,
    "Start with the proposal page that matches the task. All proposal pages are\n" +
      "   available work routes unless their own status says they are implemented,\n" +
      "   closed, superseded, or rejected.",
  );
  assertIncludes(
    proposalsIndex,
    "Compare the proposal with `../specification/` before changing code. Stop\n" +
      "   when the specification already states the behavior.",
  );
  assertIncludes(
    proposalsIndex,
    "Use [implementation-route.md](implementation-route.md) for implementation,\n" +
      "   promotion, and cleanup checks.",
  );
}

function classifyPromptRoutes(prompt) {
  return new Map(
    promptProposalRoutes(prompt).map((route) => {
      const targetRoute = route.replace("docs/proposals/", "");
      if (targetRoute === "README.md") {
        return [route, "proposal index"];
      }
      if (targetRoute === "implementation-route.md") {
        return [route, "implementation route"];
      }
      if (targetRoute === "formatter-stabilization.md") {
        return [route, "Implemented record"];
      }
      if (targetRoute === "jvm-bytecode-backend.md") {
        return [route, "Implemented record"];
      }
      if (targetRoute === "agent-language-spec-wall/repair-command.md") {
        return [route, "Implemented record"];
      }
      if (targetRoute === "reference-followups.md") {
        return [route, "Broad follow-up index"];
      }
      if (targetRoute === "agent-language-spec-wall/README.md") {
        return [route, "Exploratory inventory"];
      }
      if (targetRoute === "self-hosting-standard-library.md") {
        return [route, "Helper candidate route"];
      }
      return [route, "Selected target"];
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
    "No implementation target is selected from the current proposals.",
    "",
    "Reason:",
    "",
    "- `docs/proposals/formatter-stabilization.md`,",
    "  `docs/proposals/jvm-bytecode-backend.md`, and",
    "  `docs/proposals/agent-language-spec-wall/repair-command.md` are implemented",
    "  proposal records.",
    "- `docs/proposals/reference-followups.md` lists broad follow-up areas, but does",
    "  not define one concrete short proposal target.",
    "- `docs/proposals/agent-language-spec-wall/README.md` keeps exploratory design",
    "  wall material.",
    "- `docs/proposals/self-hosting-standard-library.md` records completed helper",
    "  migrations and says the current target is none.",
    "",
    "Useful pointers for the next proposal-selection pass:",
    "",
    "- Start at `docs/proposals/README.md`.",
    "- Use `docs/proposals/implementation-route.md` for promotion mechanics after a",
    "  concrete proposal is selected.",
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
