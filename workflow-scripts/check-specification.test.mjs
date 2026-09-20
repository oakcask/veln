import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { markdownSections, repositoryEvidenceNames, renderSpecificationFailure, specificationDocuments, validateSpecifications } from "./check-specification.mjs";

const file = "docs/specification/sample.md";
function document(body, coverage = "usage=#contract; behavior=#contract; limits=#contract") {
  return { file, text: `---\nrole: specification\nauthority: normative\nupdate-when: Sample command input and output change.\nspecification-coverage: ${coverage}\n---\n\n# Sample\n\n${body}` };
}
const valid = () => document("## Contract\n\n`veln sample PATH` reads the selected file. Missing files fail without writing output.");

// A single small contract is valid; separate sections and a particular number
// of words or headings are intentionally not a condition of acceptance.
test("accepts combined coverage and short explanatory contract", () => {
  assert.deepEqual(validateSpecifications([valid()]), { valid: true, errors: [] });
});
test("accepts tables, examples with explanations, alternative headings and nested sections", () => {
  const doc = document("## Input\n\n```sh\nveln sample input.veln\n```\n\nThe command reads a source file.\n\n## Outcomes\n\n### Fields\n\n| Field | Meaning |\n| --- | --- |\n| `status` | Failure leaves files unchanged. |", "usage=#input; behavior=#outcomes; limits=#fields");
  assert.equal(validateSpecifications([doc]).valid, true);
});
test("rejects missing concerns, missing anchors and duplicate map entries", () => {
  const result = validateSpecifications([document("## Contract\nA file is required.", "usage=#missing; usage=#contract; behavior=#contract")]);
  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /duplicate coverage/);
  assert.match(result.errors.join("\n"), /#missing does not resolve/);
  assert.match(result.errors.join("\n"), /map limits/);
});
test("rejects absent or repeated coverage maps and invalid document roles", () => {
  for (const text of [valid().text.replace(/^specification-coverage:.*\n/m, ""), valid().text.replace("specification-coverage:", "specification-coverage: usage=#contract\nspecification-coverage:")]) {
    assert.match(validateSpecifications([{ file, text }]).errors.join("\n"), /add one specification-coverage/);
  }
  assert.equal(validateSpecifications([{ file, text: valid().text.replace("role: specification", "role: reference") }]).valid, false);
});
test("rejects links, fixture paths, code-only and bare case names as explanations", () => {
  const names = new Set(["rejects_invalid_input", "sample-failure"]);
  for (const body of ["- [fixture](some.md)", "- `examples/specification/check/sample/`", "- rejects_invalid_input", "- `sample-failure`", "```sh\nveln check --json\n```", "Run `cargo test`.\nSee [the test](tests.md).", "<!-- This is an explanation hidden from readers. -->"]) {
    const result = validateSpecifications([document(`## Contract\n\n${body}`)], names);
    assert.equal(result.valid, false, body);
    assert.match(result.errors.join("\n"), /explain usage/);
  }
});
test("rejects evidence-led inventories including bare test names in behavior sections", () => {
  const doc = document("## Contract\nInput is required.\n\n- rejects_invalid_input: checks missing arguments.\n- `sample-failure` checks failures.");
  const result = validateSpecifications([doc], new Set(["rejects_invalid_input", "sample-failure"]));
  assert.equal(result.errors.length, 2);
  assert.match(result.errors[0], /replace this evidence-led item/);
});
test("permits focused references and legitimate field identifiers", () => {
  const doc = document("## Contract\nMissing files fail.\n\n- `status`: the outcome of the command.\n\n## References\n\n- `sample-failure`: command failure coverage.\n\n### Test sources\n\n- rejects_invalid_input: parsing coverage.");
  assert.equal(validateSpecifications([doc], new Set(["sample-failure", "rejects_invalid_input"])).valid, true);
});
test("rejects unlinked test-led paragraphs and table rows outside references", () => {
  const names = new Set(["rejects_invalid_input", "sample-failure"]);
  for (const body of [
    "rejects_invalid_input checks missing input.",
    "`sample-failure`, `rejects_invalid_input`: failure coverage.",
    "| Case | Coverage |\n| --- | --- |\n| rejects_invalid_input | Missing input. |",
  ]) {
    assert.equal(validateSpecifications([document(`## Contract\nMissing files fail.\n\n${body}`)], names).valid, false);
    assert.equal(validateSpecifications([document(`## Contract\nMissing files fail.\n\n## References\n${body}`)], names).valid, true);
  }
  assert.equal(validateSpecifications([document("## Contract\nMissing files fail.\n\nThe parser rejects missing input; `rejects_invalid_input` verifies this rule.")], names).valid, true);
});
test("coverage cannot point at References even when references contain prose", () => {
  const result = validateSpecifications([document("## References\nThis test demonstrates the behavior.", "usage=#references; behavior=#references; limits=#references")]);
  assert.equal(result.errors.length, 3);
});
test("routing pages are exempt from coverage, not frontmatter", () => {
  const doc = { file, text: "---\nrole: routing\nupdate-when: Command routes change.\n---\n# Commands\n\n- [Check](command-check.md): source diagnostics." };
  assert.equal(validateSpecifications([doc]).valid, true);
  assert.equal(validateSpecifications([{ file, text: "# Commands" }]).valid, false);
  assert.equal(validateSpecifications([{ file, text: doc.text.replace("role: routing", "role: routing\nspecification-coverage: usage=#commands") }]).valid, false);
});
test("rejects same-scope full pairs but permits unrelated subject names", () => {
  const result = validateSpecifications([valid(), { ...valid(), file: "docs/specification/sample-full.md" }]);
  assert.match(result.errors.join("\n"), /merge this summary\/detail pair/);
  assert.equal(validateSpecifications([{ ...valid(), file: "docs/specification/full-duplex.md" }]).valid, true);
});
test("heading parser ignores fenced headings and uses duplicate heading anchors", () => {
  const sections = markdownSections("# Title\n~~~text\n## Hidden\n~~~\n## Contract\nFirst body.\n## Contract\nSecond body.");
  assert.deepEqual(sections.map(({ anchor }) => anchor), ["title", "contract", "contract-1"]);
});
test("collects nested Markdown and actual case and Rust test identifiers", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "veln-specification-"));
  try {
    for (const dir of ["docs/specification/nested", "examples/specification/check/sample-failure", "crates/sample/src"]) fs.mkdirSync(path.join(root, dir), { recursive: true });
    fs.writeFileSync(path.join(root, "docs/specification/nested/sample.md"), valid().text);
    fs.writeFileSync(path.join(root, "examples/specification/check/sample-failure/case.toml"), "");
    fs.writeFileSync(path.join(root, "crates/sample/src/tests.rs"), "#[test]\nfn rejects_invalid_input() {}\nfn public_api() {}\n#[tokio::test]\nasync fn asynchronous_failure() {}");
    assert.deepEqual(specificationDocuments(root).map(({ file }) => file), ["docs/specification/nested/sample.md"]);
    assert.deepEqual([...repositoryEvidenceNames(root)].sort(), ["asynchronous_failure", "rejects_invalid_input", "sample-failure"]);
  } finally { fs.rmSync(root, { recursive: true, force: true }); }
});
test("failure text names the repair and reader consequence", () => {
  const message = renderSpecificationFailure([`${file}: map limits to an explanatory section`]);
  assert.match(message, /Repair the specification explanations/);
  assert.match(message, /readers need usage, behavior, and limits/);
  assert.match(message, /sample.md: map limits/);
});

test("fails empty scans and does not count references as contract explanations", () => {
  assert.equal(validateSpecifications([]).valid, false);
  const doc = document("## Contract\n\n### References\nThe fixture checks failure behavior.");
  assert.match(validateSpecifications([doc]).errors.join("\n"), /explain usage/);
});

test("rejects unlinked case-coverage prose but not the test command's contract", () => {
  const narrative = document("## Contract\nMissing files fail.\n\nThe checked cases `first_case`,\n`second_case` are the evidence for failures.");
  assert.match(validateSpecifications([narrative]).errors.join("\n"), /rewrite this test-coverage narrative/);
  const command = document("## Contract\nTests have no parameters. The test command captures output.");
  assert.equal(validateSpecifications([command]).valid, true);
});
