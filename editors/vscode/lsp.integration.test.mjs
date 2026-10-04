import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";
import { fakeDocument, loadExtension } from "./test-support.mjs";

const repo = fileURLToPath(new URL("../../", import.meta.url));
const metadata = spawnSync("cargo", ["metadata", "--locked", "--no-deps", "--format-version=1"], { cwd: repo, encoding: "utf8", timeout: 30_000 });
assert.equal(metadata.status, 0, metadata.stderr);
const binary = path.join(JSON.parse(metadata.stdout).target_directory, "debug", process.platform === "win32" ? "veln.exe" : "veln");

async function waitFor(predicate) {
  const deadline = Date.now() + 5_000;
  while (!predicate()) {
    assert(Date.now() < deadline, "Expected an observable LSP response within five seconds");
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
}

function decodeTokens(data, legend) {
  assert.equal(data.length % 5, 0);
  const tokens = [];
  let line = 0;
  let character = 0;
  for (let index = 0; index < data.length; index += 5) {
    character = data[index] === 0 ? character + data[index + 1] : data[index + 1];
    line += data[index];
    assert(data[index + 3] < legend.tokenTypes.length);
    tokens.push({ line, character, length: data[index + 2], kind: legend.tokenTypes[data[index + 3]] });
  }
  return tokens;
}

// This checks the extension/LSP boundary: provider registration, unsaved overlays,
// diagnostic conversion, and canonical package navigation. No stdlib API is under test.
test("extension providers interoperate with the real toolchain server", { timeout: 30_000 }, async (t) => {
  const project = fs.mkdtempSync(path.join(os.tmpdir(), "veln-editor-lsp-"));
  const fixture = loadExtension({
    spawn,
    workspaceFolders: [project],
    configuration: { "server.path": binary },
  });
  const context = { subscriptions: [] };
  t.after(async () => {
    const child = fixture.spawnedProcesses[0];
    try {
      if (child && child.exitCode === null && child.signalCode === null) {
        for (const subscription of context.subscriptions) subscription.dispose();
        await waitFor(() => child.exitCode !== null || child.signalCode !== null);
      }
    } finally {
      if (child && child.exitCode === null && child.signalCode === null) child.kill();
      fs.rmSync(project, { recursive: true, force: true });
    }
  });
  fs.writeFileSync(path.join(project, "veln.toml"), '[package]\nname = "app"\n\n[dependencies."example/pkg"]\npath = "vendor/pkg"\n');
  const dependency = path.join(project, "vendor", "pkg");
  fs.mkdirSync(dependency, { recursive: true });
  fs.writeFileSync(path.join(dependency, "veln.toml"), '[package]\nname = "example/pkg"\n\n[lib]\nexports = ["math.veln"]\n');
  const dependencyText = "pub fn exposed(value: Int) -> Int\r\n  value + 1\r\nend\r\n";
  fs.writeFileSync(path.join(dependency, "math.veln"), dependencyText);
  const text = 'use math from "example/pkg"\n\npub fn main() -> Int\n  math::exposed(1)\nend\n';
  const main = path.join(project, "main.veln");
  fs.writeFileSync(main, text);
  await fixture.exports.activate(context);
  const registrations = fixture.vscode._registrations;
  assert.equal(registrations.semanticTokens.length, 1);
  assert.equal(registrations.definitions.length, 1);
  const semantic = registrations.semanticTokens[0];
  const document = fakeDocument({ uri: pathToFileURL(main).href, version: 1, text });
  const tokens = decodeTokens(Array.from((await semantic.provider.provideDocumentSemanticTokens(document)).data), semantic.legend);
  assert(tokens.some((token) => token.line === 2 && token.character === 7 && token.length === 4 && token.kind === "function"));
  const location = await registrations.definitions[0].provider.provideDefinition(document, { line: 3, character: 10 });
  assert.match(location.uri.value, /^veln-pkg:\/\/\/example%2Fpkg\/snapshot\/[a-f0-9]{64}\/math\.veln$/);
  assert.equal(location.range.start.line, 0);
  assert.equal(location.range.start.character, 7);
  assert.notEqual(location.uri.toString(), location.uri.value);
  const content = await registrations.virtualDocuments[0].provider.provideTextDocumentContent(location.uri);
  assert.equal(content, dependencyText);

  const diagnostics = registrations.diagnostics[0];
  // The disk remains valid. Both changed token data and Problems entries must
  // come from the newly synchronized unsaved document.
  const invalid = fakeDocument({ uri: document.uri.toString(), version: 2, text: "fn\n" });
  const changed = decodeTokens(Array.from((await semantic.provider.provideDocumentSemanticTokens(invalid)).data), semantic.legend);
  assert.deepEqual(changed, [{ line: 0, character: 0, length: 2, kind: "keyword" }]);
  await waitFor(() => diagnostics.entries.some((entry) => entry.uri.toString() === document.uri.toString() && entry.diagnostics.length > 0));
  const published = diagnostics.entries.findLast((entry) => entry.uri.toString() === document.uri.toString() && entry.diagnostics.length > 0).diagnostics[0];
  assert.match(published.code, /^parse\./);
  assert.equal(published.source, "veln");
  assert.equal(published.severity, fixture.vscode.DiagnosticSeverity.Error);
  assert.equal(published.range.start.line, 0);
  diagnostics.entries = [];
  await semantic.provider.provideDocumentSemanticTokens(fakeDocument({ uri: document.uri.toString(), version: 3, text }));
  await waitFor(() => diagnostics.entries.some((entry) => entry.uri.toString() === document.uri.toString() && entry.diagnostics.length === 0));
  assert.equal(fs.readFileSync(main, "utf8"), text);
});
