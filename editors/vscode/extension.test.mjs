import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

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

test("syncs open documents with didOpen and later didChange", () => {
  const { exports, spawnedProcesses } = loadExtension();
  const server = new exports._test.VelnLanguageServer(
    "veln",
    ["lsp"],
    "project",
    new FakeOutputChannel(),
    () => {},
  );
  const document = fakeDocument({
    uri: "file://main.veln",
    version: 1,
    text: "fn main() -> Int\n  1\nend\n",
  });

  server.syncDocument(document);
  server.syncDocument(document);
  server.syncDocument({ ...document, version: 2, getText: () => "fn\n" });

  const messages = spawnedProcesses[0].stdin.messages.map(parseRpcMessage);
  assert.equal(messages.length, 3);
  assert.equal(messages[0].method, "initialize");
  assert.equal(messages[1].method, "textDocument/didOpen");
  assert.equal(messages[1].params.textDocument.text, document.getText());
  assert.equal(messages[2].method, "textDocument/didChange");
  assert.deepEqual(messages[2].params.contentChanges, [{ text: "fn\n" }]);
});

test("closes synced documents and publishes server diagnostics callbacks", () => {
  const diagnostics = [];
  const { exports, spawnedProcesses } = loadExtension();
  const server = new exports._test.VelnLanguageServer(
    "veln",
    ["lsp"],
    "project",
    new FakeOutputChannel(),
    (params) => diagnostics.push(params),
  );
  const document = fakeDocument({
    uri: "file://main.veln",
    version: 1,
    text: "fn\n",
  });

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
});

function loadExtension() {
  const vscode = fakeVscode();
  const spawnedProcesses = [];
  const module = { exports: {} };
  const sandbox = {
    Buffer,
    console,
    exports: module.exports,
    module,
    require(specifier) {
      if (specifier === "vscode") {
        return vscode;
      }
      if (specifier === "child_process") {
        return {
          spawn(command, args, options) {
            const process = new FakeChildProcess(command, args, options);
            spawnedProcesses.push(process);
            return process;
          },
        };
      }
      throw new Error(`unexpected require: ${specifier}`);
    },
  };
  const dirname = path.dirname(fileURLToPath(import.meta.url));
  vm.runInNewContext(fs.readFileSync(path.join(dirname, "extension.js"), "utf8"), sandbox, {
    filename: "extension.js",
  });
  return { exports: module.exports, spawnedProcesses, vscode };
}

function fakeVscode() {
  class Position {
    constructor(line, character) {
      this.line = line;
      this.character = character;
    }
  }

  class Range {
    constructor(start, end) {
      this.start = start;
      this.end = end;
    }
  }

  class Diagnostic {
    constructor(range, message, severity) {
      this.range = range;
      this.message = message;
      this.severity = severity;
    }
  }

  return {
    Diagnostic,
    DiagnosticSeverity: {
      Error: 0,
      Warning: 1,
      Information: 2,
      Hint: 3,
    },
    Position,
    Range,
    Uri: {
      parse(value) {
        return { value, toString: () => value };
      },
    },
  };
}

class FakeDiagnosticCollection {
  constructor() {
    this.entries = [];
  }

  set(uri, diagnostics) {
    this.entries.push({ uri, diagnostics });
  }
}

class FakeOutputChannel {
  append() {}
  appendLine() {}
}

class FakeChildProcess extends EventEmitter {
  constructor(command, args, options) {
    super();
    this.command = command;
    this.args = args;
    this.options = options;
    this.stdin = new FakeStdin();
    this.stdout = new EventEmitter();
    this.stderr = new EventEmitter();
  }
}

class FakeStdin {
  constructor() {
    this.messages = [];
    this.writable = true;
  }

  write(message) {
    this.messages.push(message);
  }
}

function fakeDocument({ uri, version, text }) {
  return {
    languageId: "veln",
    uri: {
      toString() {
        return uri;
      },
    },
    version,
    getText() {
      return text;
    },
  };
}

function frame(message) {
  const body = JSON.stringify(message);
  return Buffer.from(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}

function parseRpcMessage(message) {
  const bodyStart = message.indexOf("\r\n\r\n") + 4;
  return JSON.parse(message.slice(bodyStart));
}
