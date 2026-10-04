import fs from "node:fs";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repo = fileURLToPath(new URL("../../", import.meta.url));
const check = process.argv.includes("--check");
const projection = spawnSync("cargo", [
  "run", "--locked", "--quiet", "-p", "veln-repo-editor-assets", "--",
  ".", check ? "--check" : "--write",
], { cwd: repo, stdio: "inherit", timeout: 300_000 });
if (projection.error) {
  throw new Error(`Cannot generate compiler-owned editor metadata: ${projection.error.message}`);
}
if (projection.status !== 0) {
  process.exit(projection.status ?? 1);
}

const metadata = JSON.parse(fs.readFileSync(new URL("./toolchain-metadata.json", import.meta.url), "utf8"));
const escape = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const alternatives = (words) => words.map(escape).join("|");
const keywords = [...metadata.keywords, ...metadata.contextualKeywords, ...metadata.booleanLiterals];
const punctuation = [...metadata.punctuation].sort((left, right) =>
  right.length - left.length || (left < right ? -1 : left > right ? 1 : 0),
);
const rules = {
  comments: [{ name: "comment.line.number-sign.veln", match: `${escape(metadata.lineComment)}.*$` }],
  strings: [{
    name: "string.quoted.double.veln",
    begin: escape(metadata.stringDelimiter),
    end: escape(metadata.stringDelimiter),
    patterns: [{ name: "constant.character.escape.veln", match: `${escape(metadata.stringEscape)}.` }],
  }],
  numbers: [
    { name: "constant.numeric.float.veln", match: "\\b[0-9]+\\.[0-9]+\\b" },
    { name: "constant.numeric.integer.veln", match: "\\b(?:0b[01]+|0x[0-9A-Fa-f]+|[0-9]+)\\b" },
  ],
  holes: [{ name: "variable.other.hole.veln", match: "\\b_[A-Za-z0-9_]*\\b" }],
  keywords: [{ name: "keyword.control.veln", match: `\\b(${alternatives(keywords)})\\b` }],
  operators: [{ name: "keyword.operator.veln", match: alternatives(punctuation) }],
  types: [{ name: "entity.name.type.veln", match: "\\b[A-Z][A-Za-z0-9_]*\\b" }],
  identifiers: [{ name: "variable.other.veln", match: "\\b[A-Za-z][A-Za-z0-9_]*\\b" }],
};
const grammar = {
  name: "Veln",
  scopeName: "source.veln",
  patterns: Object.keys(rules).map((name) => ({ include: `#${name}` })),
  repository: Object.fromEntries(Object.entries(rules).map(([name, patterns]) => [name, { patterns }])),
};
const quote = { open: metadata.stringDelimiter, close: metadata.stringDelimiter };
const configuration = {
  comments: { lineComment: metadata.lineComment },
  brackets: metadata.brackets,
  autoClosingPairs: [
    ...metadata.brackets.map(([open, close]) => ({ open, close })),
    { ...quote, notIn: ["string", "comment"] },
  ],
  surroundingPairs: [...metadata.brackets.map(([open, close]) => ({ open, close })), quote],
};
const packageUrl = new URL("./package.json", import.meta.url);
const manifest = JSON.parse(fs.readFileSync(packageUrl, "utf8"));
// VS Code supplies standard tokens; declare only Veln-specific types and modifiers.
manifest.contributes.semanticTokenTypes = metadata.customSemanticTokenTypes.map((id) => ({ id }));
manifest.contributes.semanticTokenModifiers = metadata.customSemanticTokenModifiers.map((id) => ({ id }));

for (const [relative, value] of [
  ["./syntaxes/veln.tmLanguage.json", grammar],
  ["./language-configuration.json", configuration],
  ["./package.json", manifest],
]) {
  const url = new URL(relative, import.meta.url);
  const expected = `${JSON.stringify(value, null, 2)}\n`;
  if (check) {
    if (fs.readFileSync(url, "utf8") !== expected) {
      throw new Error(`Regenerate ${relative} with \`pnpm --filter veln-language generate:syntax\`; editor assets must match the compiler-owned metadata.`);
    }
  } else {
    fs.writeFileSync(url, expected);
  }
}
