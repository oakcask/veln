import assert from "node:assert/strict";
import test from "node:test";
import { renderDuplicationSummary } from "./summarize-code-duplication.mjs";

function report(overrides = {}) {
  return {
    statistics: {
      total: {
        sources: 3,
        lines: 400,
        clones: 2,
        duplicatedLines: 30,
        percentage: 7.5,
      },
    },
    duplicates: [
      duplicate({
        lines: 10,
        tokens: 80,
        firstFile: { name: "crates/example/src/small.rs", start: 8 },
        secondFile: { name: "crates/example/src/small.rs", start: 40 },
      }),
      duplicate({
        lines: 20,
        tokens: 120,
        firstFile: { name: "crates/example/src/first.rs", start: 12 },
        secondFile: { name: "crates/example/src/second.rs", start: 30 },
      }),
    ],
    ...overrides,
  };
}

function duplicate(overrides = {}) {
  return {
    lines: 6,
    tokens: 50,
    firstFile: { name: "crates/example/src/one.rs", start: 1 },
    secondFile: { name: "crates/example/src/two.rs", start: 10 },
    ...overrides,
  };
}

test("summarizes totals and ranks the largest clone pair first", () => {
  const summary = renderDuplicationSummary(report());

  assert.match(summary, /Rust files analyzed: 3/);
  assert.match(summary, /Duplicated lines: 30 \(7\.50%\)/);
  assert.ok(summary.indexOf("first.rs:12") < summary.indexOf("small.rs:8"));
  assert.match(summary, /advisory because some repetition is intentional/);
});

test("limits clone details and reports omitted pairs", () => {
  const summary = renderDuplicationSummary(report(), { duplicateLimit: 1 });

  assert.match(summary, /first.rs:12/);
  assert.doesNotMatch(summary, /small.rs:8/);
  assert.match(summary, /1 more clone pair\(s\) omitted/);
});

test("renders an explicit empty result", () => {
  const input = report({
    statistics: {
      total: { sources: 3, lines: 400, clones: 0, duplicatedLines: 0, percentage: 0 },
    },
    duplicates: [],
  });

  assert.match(renderDuplicationSummary(input), /No exact clone pairs were detected/);
});

test("escapes paths in the Markdown table", () => {
  const input = report({
    duplicates: [duplicate({
      firstFile: { name: "crates/a|b`c.rs", start: 2 },
    })],
  });

  assert.match(renderDuplicationSummary(input), /crates\/a\\\|b\\`c\.rs:2/);
});

test("rejects incomplete jscpd statistics", () => {
  assert.throws(
    () => renderDuplicationSummary(report({ statistics: { total: { sources: 3 } } })),
    /statistics\.total\.lines/,
  );
});

test("rejects incomplete duplicate locations", () => {
  const input = report({
    duplicates: [duplicate({ firstFile: { name: "crates/example/src/one.rs" } })],
  });

  assert.throws(() => renderDuplicationSummary(input), /file name and start line/);
});
