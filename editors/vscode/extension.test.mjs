import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { loadExtension, loadLanguageServer, initializeServer, FakeDiagnosticCollection, FakeOutputChannel, fakeDocument, frame, parseRpcMessage, serverCapabilities } from "./test-support.mjs";

test("registers semantic tokens using the connected server legend order", async () => {
  const { exports, spawnedProcesses, vscode } = loadExtension();
  const activation = exports.activate({ subscriptions: [] });
  assert.equal(vscode._registrations.semanticTokens.length, 0);
  const legend = { tokenTypes: ["number", "function", "keyword"], tokenModifiers: ["test", "declaration", "readonly"] };
  spawnedProcesses[0].stdout.emit("data", frame({ jsonrpc: "2.0", id: 1, result: {
    capabilities: { semanticTokensProvider: { full: true, legend } },
  } }));
  await activation;
  const registration = vscode._registrations.semanticTokens[0];
  assert.deepEqual(JSON.parse(JSON.stringify(registration.legend)), legend);
  assert.equal(vscode._registrations.definitions.length, 0);
  const promise = registration.provider.provideDocumentSemanticTokens(fakeDocument({ uri: "file://main.veln", version: 1, text: "42" }));
  const request = parseRpcMessage(spawnedProcesses[0].stdin.messages.at(-1));
  const data = [0, 0, 2, 0, 4];
  spawnedProcesses[0].stdout.emit("data", frame({ jsonrpc: "2.0", id: request.id, result: { data } }));
  assert.deepEqual(Array.from((await promise).data), data);
});

test("omits unsupported or malformed semantic providers", async () => {
  for (const capabilities of [{}, { semanticTokensProvider: { range: true } }, { semanticTokensProvider: { full: true, legend: { tokenTypes: [] } } }]) {
    const { exports, spawnedProcesses, vscode } = loadExtension();
    const activation = exports.activate({ subscriptions: [] });
    spawnedProcesses[0].stdout.emit("data", frame({ jsonrpc: "2.0", id: 1, result: { capabilities } }));
    await activation;
    assert.equal(vscode._registrations.semanticTokens.length, 0);
    assert.equal(vscode._registrations.definitions.length, 0);
    assert.equal(vscode._registrations.virtualDocuments.length, 1);
  }
});

test("defers document synchronization until initialized", async () => {
  const { exports, spawnedProcesses } = loadExtension();
  const server = new exports._test.VelnLanguageServer("veln", ["lsp"], "project", new FakeOutputChannel(), "off", () => {}, () => {});
  const sync = server.syncDocument(fakeDocument({ uri: "file://main.veln", version: 1, text: "fn\n" }));
  assert.deepEqual(spawnedProcesses[0].stdin.messages.map(parseRpcMessage).map((message) => message.method), ["initialize"]);
  await initializeServer(server, spawnedProcesses[0]);
  await sync;
  assert.deepEqual(spawnedProcesses[0].stdin.messages.map(parseRpcMessage).map((message) => message.method), ["initialize", "initialized", "textDocument/didOpen"]);
});

test("reports initialization failure without registering providers", async () => {
  const { exports, spawnedProcesses, vscode } = loadExtension();
  const activation = exports.activate({ subscriptions: [] });
  spawnedProcesses[0].stdout.emit("data", frame({ jsonrpc: "2.0", id: 1, error: { code: -32603, message: "initialization rejected" } }));
  await activation;
  assert.equal(vscode._registrations.semanticTokens.length, 0);
  assert.equal(vscode._registrations.definitions.length, 0);
  assert.match(vscode._registrations.outputs[0].lines.at(-1), /Failed to initialize.*initialization rejected/);
});

test("disposal during initialization prevents late provider registration", async () => {
  const { exports, spawnedProcesses, vscode } = loadExtension();
  const context = { subscriptions: [] };
  const activation = exports.activate(context);
  context.subscriptions.at(-1).dispose();
  spawnedProcesses[0].stdout.emit("data", frame({ jsonrpc: "2.0", id: 1, result: { capabilities: serverCapabilities } }));
  await activation;
  assert.equal(vscode._registrations.semanticTokens.length, 0);
  assert.equal(vscode._registrations.virtualDocuments.length, 0);
});

test("maps published LSP diagnostics into a VSCode diagnostic collection", () => {
  const { exports, vscode } = loadExtension();
  const collection = new FakeDiagnosticCollection();

  exports._test.applyDiagnostics(collection, {
    uri: "file://main.veln",
    diagnostics: [
      {
        range: {
          start: { line: 1, character: 2 },
          end: { line: 1, character: 7 },
        },
        severity: 1,
        code: "parse.expected_item",
        source: "veln",
        message: "expected a function or test declaration",
      },
    ],
  });

  assert.equal(collection.entries.length, 1);
  assert.equal(collection.entries[0].uri.value, "file://main.veln");
  assert.equal(collection.entries[0].diagnostics.length, 1);

  const [diagnostic] = collection.entries[0].diagnostics;
  assert.equal(diagnostic.message, "expected a function or test declaration");
  assert.equal(diagnostic.severity, vscode.DiagnosticSeverity.Error);
  assert.equal(diagnostic.code, "parse.expected_item");
  assert.equal(diagnostic.source, "veln");
  assert.equal(diagnostic.range.start.line, 1);
  assert.equal(diagnostic.range.start.character, 2);
  assert.equal(diagnostic.range.end.line, 1);
  assert.equal(diagnostic.range.end.character, 7);
});

test("uses veln as the default diagnostic source", () => {
  const { exports } = loadExtension();
  const collection = new FakeDiagnosticCollection();

  exports._test.applyDiagnostics(collection, {
    uri: "file://main.veln",
    diagnostics: [
      {
        range: {
          start: { line: 0, character: 0 },
          end: { line: 0, character: 0 },
        },
        severity: 2,
        code: "type.mismatch",
        message: "type mismatch",
      },
    ],
  });

  const [diagnostic] = collection.entries[0].diagnostics;
  assert.equal(diagnostic.source, "veln");
});

test("maps all LSP diagnostic severities", () => {
  const { exports, vscode } = loadExtension();

  assert.equal(
    exports._test.toDiagnosticSeverity(1),
    vscode.DiagnosticSeverity.Error,
  );
  assert.equal(
    exports._test.toDiagnosticSeverity(2),
    vscode.DiagnosticSeverity.Warning,
  );
  assert.equal(
    exports._test.toDiagnosticSeverity(3),
    vscode.DiagnosticSeverity.Information,
  );
  assert.equal(
    exports._test.toDiagnosticSeverity(4),
    vscode.DiagnosticSeverity.Hint,
  );
  assert.equal(
    exports._test.toDiagnosticSeverity(undefined),
    vscode.DiagnosticSeverity.Error,
  );
});

test("syncs open documents with didOpen and later didChange", async () => {
  const { server, spawnedProcesses } = await loadLanguageServer();
  const document = fakeDocument({
    uri: "file://main.veln",
    version: 1,
    text: "fn main() -> Int\n  1\nend\n",
  });

  server.syncDocument(document);
  server.syncDocument(document);
  server.syncDocument({ ...document, version: 2, getText: () => "fn\n" });

  const messages = spawnedProcesses[0].stdin.messages.map(parseRpcMessage);
  assert.equal(messages.length, 4);
  assert.equal(messages[0].method, "initialize");
  assert.equal(messages[2].method, "textDocument/didOpen");
  assert.equal(messages[2].params.textDocument.text, document.getText());
  assert.equal(messages[3].method, "textDocument/didChange");
  assert.deepEqual(messages[3].params.contentChanges, [{ text: "fn\n" }]);
});

test("follows a dependency definition through the virtual document request", async () => {
  const { exports, server, spawnedProcesses } = await loadLanguageServer();
  const document = fakeDocument({
    uri: "file://project/main.veln",
    version: 1,
    text: "use math from \"example/pkg\"\n\npub fn main() -> Int\n  math::exposed(1)\nend\n",
  });
  const virtualUri =
    "veln-pkg:///example%2Fpkg/snapshot/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/math.veln";

  const definitionPromise = server.definition(document, { line: 3, character: 10 });
  let messages = spawnedProcesses[0].stdin.messages.map(parseRpcMessage);
  assert.equal(messages.at(-1).method, "textDocument/definition");
  assert.equal(messages.at(-1).params.textDocument.uri, document.uri.toString());
  spawnedProcesses[0].stdout.emit(
    "data",
    frame({
      jsonrpc: "2.0",
      id: messages.at(-1).id,
      result: {
        uri: virtualUri,
        range: {
          start: { line: 0, character: 7 },
          end: { line: 0, character: 14 },
        },
      },
    }),
  );
  const definition = await definitionPromise;
  assert.equal(definition.uri, virtualUri);

  const exactText = "pub fn exposed(value: Int) -> Int\r\n  value + 1\r\nend\r\n";
  const readPromise = server.virtualDocument({ toString: () => definition.uri });
  messages = spawnedProcesses[0].stdin.messages.map(parseRpcMessage);
  assert.equal(messages.at(-1).method, "veln/virtualDocument");
  assert.deepEqual(
    JSON.parse(JSON.stringify(messages.at(-1).params)),
    { uri: virtualUri },
  );
  spawnedProcesses[0].stdout.emit(
    "data",
    frame({ jsonrpc: "2.0", id: messages.at(-1).id, result: exactText }),
  );
  assert.equal(await readPromise, exactText);

  const location = exports._test.toLocation(definition);
  assert.notEqual(location.uri.toString(), virtualUri);
  assert.equal(location.range.start.line, 0);
  assert.equal(location.range.start.character, 7);
});

test("registers the veln-pkg content provider with canonical URI lookup", async () => {
  const { exports, spawnedProcesses, vscode } = loadExtension();
  const context = { subscriptions: [] };
  const activation = exports.activate(context);
  spawnedProcesses[0].stdout.emit("data", frame({ jsonrpc: "2.0", id: 1, result: { capabilities: serverCapabilities } }));
  await activation;

  assert.equal(vscode._registrations.virtualDocuments.length, 1);
  const registration = vscode._registrations.virtualDocuments[0];
  assert.equal(registration.scheme, "veln-pkg");
  assert.equal(vscode._registrations.definitions.length, 1);

  const uri = vscode.Uri.parse(
    "veln-pkg:///example%2Fpkg/snapshot/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/math.veln",
  );
  assert.equal(
    uri.toString(),
    "veln-pkg:///example/pkg/snapshot/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/math.veln",
  );

  const document = fakeDocument({
    uri: "file://project/main.veln",
    version: 1,
    text: "use math from \"example/pkg\"\n\npub fn main() -> Int\n  math::exposed(1)\nend\n",
  });
  const definitionPromise = vscode._registrations.definitions[0].provider.provideDefinition(
    document,
    { line: 3, character: 10 },
  );
  let messages = spawnedProcesses[0].stdin.messages.map(parseRpcMessage);
  const definitionRequest = messages.at(-1);
  assert.equal(definitionRequest.method, "textDocument/definition");
  spawnedProcesses[0].stdout.emit(
    "data",
    frame({
      jsonrpc: "2.0",
      id: definitionRequest.id,
      result: {
        uri: uri.value,
        range: {
          start: { line: 0, character: 7 },
          end: { line: 0, character: 14 },
        },
      },
    }),
  );
  assert.equal((await definitionPromise).uri.toString(), uri.toString());

  const contentPromise = registration.provider.provideTextDocumentContent(uri);
  messages = spawnedProcesses[0].stdin.messages.map(parseRpcMessage);
  const request = messages.at(-1);
  assert.equal(request.method, "veln/virtualDocument");
  assert.deepEqual(request.params, { uri: uri.value });
  spawnedProcesses[0].stdout.emit(
    "data",
    frame({ jsonrpc: "2.0", id: request.id, result: "exact source\r\n" }),
  );
  assert.equal(await contentPromise, "exact source\r\n");
});

test("initializes the language server with workspace identity", () => {
  const { exports, spawnedProcesses } = loadExtension();
  new exports._test.VelnLanguageServer(
    "veln",
    ["lsp"],
    "project-root",
    new FakeOutputChannel(),
    "off",
    () => {},
    () => {},
  );

  const [message] = spawnedProcesses[0].stdin.messages.map(parseRpcMessage);
  assert.equal(message.method, "initialize");
  assert.equal(message.params.rootUri, "file://project-root");
  assert.deepEqual(message.params.workspaceFolders, [
    { uri: "file://project-root", name: "project-root" },
  ]);
});

test("initializes the language server with all workspace folders", () => {
  const { exports, spawnedProcesses } = loadExtension();
  new exports._test.VelnLanguageServer(
    "veln",
    ["lsp"],
    "alpha",
    new FakeOutputChannel(),
    "off",
    () => {},
    () => {},
    ["alpha", "beta"],
  );

  const [message] = spawnedProcesses[0].stdin.messages.map(parseRpcMessage);
  assert.equal(message.method, "initialize");
  assert.equal(message.params.rootUri, "file://alpha");
  assert.deepEqual(message.params.workspaceFolders, [
    { uri: "file://alpha", name: "alpha" },
    { uri: "file://beta", name: "beta" },
  ]);
});

test("uses explicit language roots for rootUri", () => {
  const { exports } = loadExtension();

  const params = exports._test.initializeParams("repo", ["project"]);

  assert.equal(params.rootUri, "file://project");
  assert.deepEqual(JSON.parse(JSON.stringify(params.workspaceFolders)), [
    { uri: "file://project", name: "project" },
  ]);
});

test("omits workspace identity when no workspace folder is active", () => {
  const { exports } = loadExtension();

  assert.deepEqual(JSON.parse(JSON.stringify(exports._test.initializeParams(undefined))), {
    capabilities: {},
  });
});

test("omits workspace identity when workspace folders are explicitly empty", () => {
  const { exports } = loadExtension();

  assert.deepEqual(JSON.parse(JSON.stringify(exports._test.initializeParams("repo", []))), {
    capabilities: {},
  });
});

test("uses workspace folders with veln manifests as language roots", (t) => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), "veln-vscode-"));
  t.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  const alpha = path.join(temp, "alpha");
  const beta = path.join(temp, "beta");
  fs.mkdirSync(alpha);
  fs.mkdirSync(beta);
  fs.writeFileSync(path.join(alpha, "veln.toml"), "[package]\nname = \"alpha\"\n");
  const { exports } = loadExtension({
    workspaceFolders: [alpha, beta],
  });

  assert.deepEqual(exports._test.velnWorkspaceFolderPaths(), [alpha]);
});

test("uses nested manifest folders as language roots", (t) => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), "veln-vscode-"));
  t.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  const repo = path.join(temp, "repo");
  const alpha = path.join(repo, "examples", "alpha");
  const beta = path.join(repo, "examples", "beta");
  const vendor = path.join(beta, "vendor", "foo");
  fs.mkdirSync(alpha, { recursive: true });
  fs.mkdirSync(vendor, { recursive: true });
  fs.writeFileSync(path.join(alpha, "veln.toml"), "[package]\nname = \"alpha\"\n");
  fs.writeFileSync(path.join(beta, "veln.toml"), "[package]\nname = \"beta\"\n");
  fs.writeFileSync(path.join(vendor, "veln.toml"), "[package]\nname = \"foo\"\n");
  const { exports } = loadExtension({
    workspaceFolders: [repo],
  });

  assert.deepEqual(exports._test.velnWorkspaceFolderPaths(), [alpha, beta]);
});

test("uses first manifestless source subtree as an anonymous language root", (t) => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), "veln-vscode-"));
  t.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  const alpha = path.join(temp, "alpha");
  const beta = path.join(temp, "beta");
  fs.mkdirSync(alpha);
  fs.mkdirSync(path.join(beta, "src"), { recursive: true });
  fs.writeFileSync(path.join(beta, "src", "main.veln"), "fn main() -> Int\n  1\nend\n");
  const { exports } = loadExtension({
    workspaceFolders: [alpha, beta],
  });

  assert.deepEqual(exports._test.velnWorkspaceFolderPaths(), [path.join(beta, "src")]);
});

test("closes synced documents and publishes server diagnostics callbacks", async () => {
  const diagnostics = [];
  let cleared = false;
  const { exports, spawnedProcesses } = loadExtension();
  const server = new exports._test.VelnLanguageServer(
    "veln",
    ["lsp"],
    "project",
    new FakeOutputChannel(),
    "off",
    (params) => diagnostics.push(params),
    () => {
      cleared = true;
    },
  );
  const document = fakeDocument({
    uri: "file://main.veln",
    version: 1,
    text: "fn\n",
  });

  await initializeServer(server, spawnedProcesses[0]);
  server.syncDocument(document);
  server.closeDocument(document);
  server.closeDocument(document);
  spawnedProcesses[0].stdout.emit(
    "data",
    frame({
      jsonrpc: "2.0",
      method: "textDocument/publishDiagnostics",
      params: { uri: document.uri.toString(), diagnostics: [] },
    }),
  );

  const messages = spawnedProcesses[0].stdin.messages.map(parseRpcMessage);
  assert.equal(messages.at(-1).method, "textDocument/didClose");
  assert.equal(messages.filter((message) => message.method === "textDocument/didClose").length, 1);
  assert.deepEqual(JSON.parse(JSON.stringify(diagnostics)), [
    { uri: document.uri.toString(), diagnostics: [] },
  ]);
  assert.equal(cleared, false);
});

test("traces compact protocol messages and redacts verbose document text", async () => {
  const { exports, spawnedProcesses } = loadExtension();
  const output = new FakeOutputChannel();
  const server = new exports._test.VelnLanguageServer(
    "veln",
    ["lsp"],
    "project",
    output,
    "messages",
    () => {},
    () => {},
  );
  const document = fakeDocument({
    uri: "file://main.veln",
    version: 1,
    text: "fn main() -> Int\n  1\nend\n",
  });

  await initializeServer(server, spawnedProcesses[0]);
  server.syncDocument(document);

  assert.match(output.lines[0], /^\[lsp\] starting server command=/);
  assert(output.lines.includes("[lsp:client] initialize id=1"));
  assert(output.lines.includes("[lsp:client] textDocument/didOpen"));
  assert(output.lines.includes("[lsp:server] response id=1 ok"));

  const verbose = exports._test.summarizeJson({
    params: {
      textDocument: {
        text: "fn main() -> Int\n  1\nend\n",
      },
    },
  });
  assert.match(verbose, /"<\d+ chars>"/);
  assert(!verbose.includes("fn main()"));
});

test("keeps reading messages after malformed server JSON", () => {
  const diagnostics = [];
  const { exports, spawnedProcesses } = loadExtension();
  const output = new FakeOutputChannel();
  new exports._test.VelnLanguageServer(
    "veln",
    ["lsp"],
    "project",
    output,
    "off",
    (params) => diagnostics.push(params),
    () => {},
  );

  spawnedProcesses[0].stdout.emit(
    "data",
    Buffer.from("Content-Length: 1\r\n\r\n{"),
  );
  spawnedProcesses[0].stdout.emit(
    "data",
    frame({
      jsonrpc: "2.0",
      method: "textDocument/publishDiagnostics",
      params: { uri: "file://main.veln", diagnostics: [] },
    }),
  );

  assert.equal(output.lines.length, 1);
  assert.match(output.lines[0], /^Failed to parse Veln language server message:/);
  assert.deepEqual(JSON.parse(JSON.stringify(diagnostics)), [
    { uri: "file://main.veln", diagnostics: [] },
  ]);
});

test("clears diagnostics when the server exits", () => {
  let cleared = false;
  const { exports, spawnedProcesses } = loadExtension();
  new exports._test.VelnLanguageServer(
    "veln",
    ["lsp"],
    "project",
    new FakeOutputChannel(),
    "off",
    () => {},
    () => {
      cleared = true;
    },
  );

  spawnedProcesses[0].stdout.emit("data", frame({ jsonrpc: "2.0", id: 1, result: {} }));
  spawnedProcesses[0].emit("exit", 1, null);

  assert.equal(cleared, true);
});
