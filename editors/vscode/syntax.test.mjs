import assert from "node:assert/strict";
import fs from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import textmate from "vscode-textmate";
import oniguruma from "vscode-oniguruma";

const require = createRequire(import.meta.url);
const wasm = fs.readFileSync(require.resolve("vscode-oniguruma/release/onig.wasm"));
await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
const registry = new textmate.Registry({
  onigLib: Promise.resolve({
    createOnigScanner: (patterns) => new oniguruma.OnigScanner(patterns),
    createOnigString: (text) => new oniguruma.OnigString(text),
  }),
  loadGrammar: async () => JSON.parse(fs.readFileSync(new URL("./syntaxes/veln.tmLanguage.json", import.meta.url), "utf8")),
});
const grammar = await registry.loadGrammar("source.veln");
const metadata = JSON.parse(fs.readFileSync(new URL("./toolchain-metadata.json", import.meta.url), "utf8"));

function classify(scopes) {
  if (scopes.some((scope) => scope.startsWith("comment."))) return "comment";
  if (scopes.some((scope) => scope.startsWith("string."))) return "string";
  if (scopes.some((scope) => scope.startsWith("constant.numeric."))) return "number";
  if (scopes.includes("variable.other.hole.veln")) return "hole";
  if (scopes.includes("keyword.control.veln")) return "keyword";
  if (scopes.includes("keyword.operator.veln")) return "operator";
  if (scopes.includes("entity.name.type.veln")) return "type";
  if (scopes.includes("variable.other.veln")) return "identifier";
  return undefined;
}

function tokenize(source, state = textmate.INITIAL) {
  const result = grammar.tokenizeLine(source, state);
  const tokens = [];
  for (const token of result.tokens) {
    const start = token.startIndex;
    const end = Math.min(token.endIndex, source.length);
    if (start === end) continue;
    const kind = classify(token.scopes);
    if (!kind && source.slice(start, end).trim() === "") continue;
    assert(kind, `Unclassified TextMate region ${JSON.stringify(source.slice(start, end))}`);
    const previous = tokens.at(-1);
    if (previous?.kind === "string" && kind === "string" && previous.end === start) {
      previous.end = end;
    } else {
      tokens.push({ start, end, kind });
    }
  }
  return { tokens, state: result.ruleStack };
}

test("TextMate token boundaries and classes follow the compiled lexer projection", () => {
  for (const fixture of metadata.lexicalFixtures) {
    assert.deepEqual(tokenize(fixture.source).tokens, fixture.tokens, fixture.source);
  }
});

test("strings contain comments and escaped quotes without consuming following syntax", () => {
  const source = '"# \\" fn" + 42 # tail';
  const result = tokenize(source).tokens;
  assert.deepEqual(result.map((token) => [source.slice(token.start, token.end), token.kind]), [
    ['"# \\" fn"', "string"], ["+", "operator"], ["42", "number"], ["# tail", "comment"],
  ]);
});

test("incomplete strings retain state while line comments end at the newline", () => {
  const open = tokenize('"unfinished');
  assert.deepEqual(open.tokens, [{ start: 0, end: 11, kind: "string" }]);
  assert.deepEqual(tokenize('fn # still string', open.state).tokens, [{ start: 0, end: 17, kind: "string" }]);
  const comment = tokenize("# fn");
  assert.deepEqual(tokenize("fn", comment.state).tokens, [{ start: 0, end: 2, kind: "keyword" }]);
});

test("custom semantic declarations follow compiler metadata", () => {
  const manifest = JSON.parse(fs.readFileSync(new URL("./package.json", import.meta.url), "utf8"));
  assert.deepEqual(manifest.contributes.semanticTokenModifiers.map((modifier) => modifier.id), metadata.customSemanticTokenModifiers);
  assert.deepEqual(manifest.contributes.semanticTokenTypes.map((kind) => kind.id), metadata.customSemanticTokenTypes);
});
