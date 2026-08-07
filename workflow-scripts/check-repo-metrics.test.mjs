import assert from "node:assert/strict";
import test from "node:test";
import {
  evaluateRepositoryPolicy,
  numberedSplitFiles,
  renderGitHubAnnotation,
} from "./check-repo-metrics.mjs";

function report(overrides = {}) {
  return {
    schema_version: "veln-repo-metrics-json/v0",
    tool: { name: "veln-repo-metrics", version: "0.1.0" },
    inputs: { roots: ["crates"], abc_threshold: 30, file_line_threshold: 700 },
    summary: { rust_file_count: 3, finding_count: 2 },
    files: ["crates/sample/src/lib.rs", "crates/sample/src/item.rs", "crates/sample/src/large.rs"],
    findings: [
      {
        kind: "abc_complexity",
        path: "crates/sample/src/lib.rs",
        line: 4,
        subject: "parse",
        abc: { assignments: 12, branches: 18, conditionals: 21, magnitude: 30.4 },
      },
      {
        kind: "file_line_count",
        path: "crates/sample/src/large.rs",
        line: 1,
        lines: 701,
      },
    ],
    dependency_graph: {
      file_count: 3,
      edge_count: 1,
      hotspots: [{ path: "crates/sample/src/lib.rs", incoming: 1, outgoing: 1, pressure: 1 }],
      cycles: [],
    },
    ...overrides,
  };
}

test("keeps metric findings advisory", () => {
  const result = evaluateRepositoryPolicy(report());

  assert.equal(result.valid, true);
  assert.deepEqual(result.annotations.map(({ level, title }) => ({ level, title })), [
    { level: "warning", title: "High ABC complexity" },
    { level: "warning", title: "Large Rust file" },
  ]);
  assert.match(result.summary, /Highest dependency pressure/);
});

test("blocks dependency cycles with actionable evidence", () => {
  const cycle = ["crates/sample/src/lib.rs", "crates/sample/src/item.rs"];
  const input = report({
    dependency_graph: {
      file_count: 3,
      edge_count: 2,
      hotspots: [],
      cycles: [cycle],
    },
  });
  const result = evaluateRepositoryPolicy(input);

  assert.equal(result.valid, false);
  assert.equal(result.annotations[0].level, "error");
  assert.match(result.annotations[0].message, /Remove this dependency cycle before merging/);
  assert.match(result.annotations[0].message, /lib\.rs -> crates\/sample\/src\/item\.rs -> crates\/sample\/src\/lib\.rs/);
});

test("detects only numbered suffix series in one directory and prefix", () => {
  assert.deepEqual(numberedSplitFiles([
    "crates/sample/src/parser01.rs",
    "crates/sample/src/parser02.rs",
    "crates/sample/src/sha256.rs",
    "crates/sample/src/nested/parser03.rs",
    "crates/sample/src/nested/parser04.rs",
    "crates/sample/src/partitions.rs",
  ]), [
    "crates/sample/src/nested/parser03.rs",
    "crates/sample/src/nested/parser04.rs",
    "crates/sample/src/parser01.rs",
    "crates/sample/src/parser02.rs",
  ]);
});

test("prioritizes blocking annotations and reports truncation", () => {
  const input = report({
    files: ["crates/sample/src/part01.rs", "crates/sample/src/part02.rs"],
  });
  const result = evaluateRepositoryPolicy(input, { maxAnnotations: 1 });

  assert.equal(result.valid, false);
  assert.equal(result.annotations[0].title, "Numbered split file");
  assert.equal(result.omittedAnnotationCount, 3);
});

test("escapes GitHub annotation properties and messages", () => {
  const output = renderGitHubAnnotation({
    level: "warning",
    file: "a:b,c%.rs",
    line: 2,
    title: "A:B,C%",
    message: "check 100%\r\nnext",
  });

  assert.equal(
    output,
    "::warning file=a%3Ab%2Cc%25.rs,line=2,title=A%3AB%2CC%25::check 100%25%0D%0Anext",
  );
});

test("rejects an unsupported report schema", () => {
  assert.throws(
    () => evaluateRepositoryPolicy(report({ schema_version: "veln-repo-metrics-json/v999" })),
    /expected schema_version/,
  );
});
