import fs from "node:fs";
import process from "node:process";

const grammarUrl = new URL("./syntaxes/veln.tmLanguage.json", import.meta.url);
const specificationUrl = new URL(
  "../../docs/specification/source-surface-executable.pl",
  import.meta.url,
);
const contextualKeywords = ["validate", "satisfy", "true", "false"];

const specification = fs.readFileSync(specificationUrl, "utf8");
const reservedKeywords = Array.from(
  specification.matchAll(/^keyword_kind\("([a-z]+)", [a-z]+\)\.$/gm),
  (match) => match[1],
);

if (reservedKeywords.length === 0) {
  throw new Error("No keywords found in the executable syntax specification.");
}

const keywordPattern = `\\b(${[
  ...reservedKeywords,
  ...contextualKeywords,
].join("|")})\\b`;
const grammar = JSON.parse(fs.readFileSync(grammarUrl, "utf8"));
const keywordRule = grammar.repository.keywords.patterns.find(
  (pattern) => pattern.name === "keyword.control.veln",
);

if (keywordRule === undefined) {
  throw new Error("TextMate grammar has no Veln keyword rule to update.");
}

if (process.argv.includes("--check")) {
  if (keywordRule.match !== keywordPattern) {
    throw new Error(
      "TextMate keyword list is stale; run `pnpm --filter veln-language generate:syntax` so editor highlighting follows the executable syntax specification.",
    );
  }
} else {
  keywordRule.match = keywordPattern;
  fs.writeFileSync(grammarUrl, `${JSON.stringify(grammar, null, 2)}\n`);
}
