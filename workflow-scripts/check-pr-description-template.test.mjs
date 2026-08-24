import assert from "node:assert/strict";
import test from "node:test";
import { validatePullRequestDescription } from "./check-pr-description-template.mjs";

test("accepts the PR template sections with optional stack preface", () => {
  const result = validatePullRequestDescription(
    [
      "Stack: 2 of 3. Depends on #123.",
      "",
      "## Intent",
      "",
      "Explain why this change is needed.",
      "",
      "## Consequences",
      "",
      "None.",
      "",
      "## Risks",
      "",
      "Review the boundary behavior.",
      "",
      "## Verification",
      "",
      "- pnpm test",
    ].join("\n"),
  );

  assert.equal(result.valid, true);
});

test("accepts a breaking PR title when the description explains the breaking change", () => {
  const result = validatePullRequestDescription(
    [
      "## Intent",
      "",
      "Replace the compatibility aliases with the semantic fields.",
      "",
      "## Consequences",
      "",
      "Consumers must read the semantic diagnostic fields.",
      "",
      "BREAKING CHANGE: Generated diagnostics no longer emit the old numbered aliases.",
      "",
      "## Risks",
      "",
      "Downstream callers may still read the removed aliases.",
      "",
      "## Verification",
      "",
      "- pnpm test",
    ].join("\n"),
    { title: "refactor(core)!: remove legacy diagnostic aliases" },
  );

  assert.equal(result.valid, true);
});

test("rejects a breaking PR title without a BREAKING CHANGE line", () => {
  const result = validatePullRequestDescription(
    [
      "## Intent",
      "",
      "Replace the compatibility aliases with the semantic fields.",
      "",
      "## Consequences",
      "",
      "Consumers must read the semantic diagnostic fields.",
      "",
      "## Risks",
      "",
      "This is a breaking API cleanup.",
      "",
      "## Verification",
      "",
      "- pnpm test",
    ].join("\n"),
    { title: "refactor(core)!: remove legacy diagnostic aliases" },
  );

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "PRs marked breaking with ! in the title must include a BREAKING CHANGE: line describing the compatibility impact.",
  ]);
});

test("rejects an empty BREAKING CHANGE line for a breaking PR title", () => {
  const result = validatePullRequestDescription(
    [
      "## Intent",
      "",
      "Replace the compatibility aliases with the semantic fields.",
      "",
      "## Consequences",
      "",
      "BREAKING CHANGE:",
      "",
      "## Risks",
      "",
      "Downstream callers may still read the removed aliases.",
      "",
      "## Verification",
      "",
      "- pnpm test",
    ].join("\n"),
    { title: "refactor(core)!: remove legacy diagnostic aliases" },
  );

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "PRs marked breaking with ! in the title must include a BREAKING CHANGE: line describing the compatibility impact.",
  ]);
});

test("rejects the old summary and tests section format", () => {
  const result = validatePullRequestDescription(
    [
      "Stack: 2 of 3. Depends on #280.",
      "",
      "Summary:",
      "- extract counter-subject support-collision counting into named predicates",
      "",
      "Tests:",
      "- pnpm lint",
    ].join("\n"),
  );

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "Use exactly these H2 sections in this order: ## Intent, ## Consequences, ## Risks, ## Verification.",
  ]);
});

test("rejects missing, extra, or reordered H2 sections", () => {
  const result = validatePullRequestDescription(
    [
      "## Intent",
      "Motivation.",
      "",
      "## Verification",
      "- pnpm test",
      "",
      "## Risks",
      "Unexpected order.",
      "",
      "## Notes",
      "Extra section.",
    ].join("\n"),
  );

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "Use exactly these H2 sections in this order: ## Intent, ## Consequences, ## Risks, ## Verification.",
  ]);
});

test("rejects template sections left with only comments", () => {
  const result = validatePullRequestDescription(
    [
      "## Intent",
      "<!-- Explain the motivation. -->",
      "",
      "## Consequences",
      "None.",
      "",
      "## Risks",
      "None.",
      "",
      "## Verification",
      "- pnpm test",
    ].join("\n"),
  );

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, ["## Intent must contain reviewer-facing content."]);
});
