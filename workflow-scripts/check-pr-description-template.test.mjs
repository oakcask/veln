import assert from "node:assert/strict";
import test from "node:test";
import { validatePullRequestDescription } from "./check-pr-description-template.mjs";

const requiredSectionError =
  "Use exactly these H2 sections in this order: ## Intent, ## Consequences, ## Risks, " +
  "## Compliance and Revisit Triggers.";

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
      "## Compliance and Revisit Triggers",
      "",
      "Boundary assertions enforce the contract. Revisit it when callers reject empty input.",
    ].join("\n"),
  );

  assert.equal(result.valid, true);
});

test("accepts a breaking PR title when the description explains the breaking change", () => {
  const result = validatePullRequestDescription(
    breakingDescription([
      "Consumers must read the semantic diagnostic fields.",
      "",
      "BREAKING CHANGE: Generated diagnostics no longer emit the old numbered aliases.",
    ]),
    { title: "refactor(core)!: remove legacy diagnostic aliases" },
  );

  assert.equal(result.valid, true);
});

test("rejects a breaking PR title without a BREAKING CHANGE line", () => {
  const result = validatePullRequestDescription(
    breakingDescription("Consumers must read the semantic diagnostic fields.", {
      risks: "This is a breaking API cleanup.",
    }),
    { title: "refactor(core)!: remove legacy diagnostic aliases" },
  );

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "PRs marked breaking with ! in the title must include a BREAKING CHANGE: line describing the compatibility impact.",
  ]);
});

test("rejects an empty BREAKING CHANGE line for a breaking PR title", () => {
  const result = validatePullRequestDescription(
    breakingDescription("BREAKING CHANGE:"),
    { title: "refactor(core)!: remove legacy diagnostic aliases" },
  );

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [
    "PRs marked breaking with ! in the title must include a BREAKING CHANGE: line describing the compatibility impact.",
  ]);
});

function breakingDescription(
  consequences,
  { risks = "Downstream callers may still read the removed aliases." } = {},
) {
  const consequenceLines = Array.isArray(consequences) ? consequences : [consequences];
  return [
    "## Intent",
    "",
    "Replace the compatibility aliases with the semantic fields.",
    "",
    "## Consequences",
    "",
    ...consequenceLines,
    "",
    "## Risks",
    "",
    risks,
    "",
    "## Compliance and Revisit Triggers",
    "",
    "Semantic assertions enforce the contract. Revisit it if consumers need aliases.",
  ].join("\n");
}

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
  assert.deepEqual(result.errors, [requiredSectionError]);
});

test("rejects the former Verification heading", () => {
  const result = validatePullRequestDescription(
    [
      "## Intent",
      "Motivation.",
      "",
      "## Consequences",
      "None.",
      "",
      "## Risks",
      "None.",
      "",
      "## Verification",
      "The boundary assertion enforces the decision.",
    ].join("\n"),
  );

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [requiredSectionError]);
});

test("rejects missing, extra, or reordered H2 sections", () => {
  const result = validatePullRequestDescription(
    [
      "## Intent",
      "Motivation.",
      "",
      "## Compliance and Revisit Triggers",
      "Assertions enforce the decision. Revisit it when the input contract changes.",
      "",
      "## Risks",
      "Unexpected order.",
      "",
      "## Notes",
      "Extra section.",
    ].join("\n"),
  );

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, [requiredSectionError]);
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
      "## Compliance and Revisit Triggers",
      "Assertions enforce the decision. Revisit it when the input contract changes.",
    ].join("\n"),
  );

  assert.equal(result.valid, false);
  assert.deepEqual(result.errors, ["## Intent must contain reviewer-facing content."]);
});
