import { EventEmitter } from "node:events";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

export async function loadLanguageServer() {
  const fixture = loadExtension();
  const server = new fixture.exports._test.VelnLanguageServer(
    "veln",
    ["lsp"],
    "project",
    new FakeOutputChannel(),
    "off",
    () => {},
    () => {},
  );
  await initializeServer(server, fixture.spawnedProcesses[0]);
  return { ...fixture, server };
}

export function loadExtension(fixtureOptions = {}) {
  const options = fixtureOptions;
  const vscode = fakeVscode(options);
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
            const process = fixtureOptions.spawn
              ? fixtureOptions.spawn(command, args, options)
              : new FakeChildProcess(command, args, options);
            spawnedProcesses.push(process);
            return process;
          },
        };
      }
      if (specifier === "fs") {
        return fs;
      }
      if (specifier === "path") {
        return path;
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

function fakeVscode(options = {}) {
  const registrations = {
    definitions: [],
    virtualDocuments: [],
    semanticTokens: [],
    diagnostics: [],
    outputs: [],
  };
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

  class Location {
    constructor(uri, range) {
      this.uri = uri;
      this.range = range;
    }
  }

  class SemanticTokensLegend {
    constructor(tokenTypes, tokenModifiers) {
      this.tokenTypes = tokenTypes;
      this.tokenModifiers = tokenModifiers;
    }
  }

  class SemanticTokens {
    constructor(data) {
      this.data = data;
    }
  }

  const disposable = () => ({ dispose() {} });

  return {
    _registrations: registrations,
    Diagnostic,
    Location,
    DiagnosticSeverity: {
      Error: 0,
      Warning: 1,
      Information: 2,
      Hint: 3,
    },
    Position,
    Range,
    SemanticTokens,
    SemanticTokensLegend,
    Uri: {
      file(value) {
        const normalized = value.replaceAll("\\", "/");
        const uri = normalized.startsWith("/")
          ? `file://${normalized}`
          : `file://${normalized}`;
        return { value: uri, fsPath: value, toString: () => uri };
      },
      parse(value) {
        if (value.startsWith("veln-pkg:///")) {
          const displayed = value.replaceAll("%2F", "/");
          return { value, toString: () => displayed };
        }
        return { value, toString: () => value };
      },
    },
    workspace: {
      workspaceFolders: (options.workspaceFolders ?? []).map((folder) => ({
        uri: { fsPath: folder },
      })),
      textDocuments: options.documents ?? [],
      getConfiguration() {
        return { get(name, fallback) { return options.configuration?.[name] ?? fallback; } };
      },
      onDidOpenTextDocument: disposable,
      onDidChangeTextDocument: disposable,
      onDidCloseTextDocument: disposable,
      registerTextDocumentContentProvider(scheme, provider) {
        registrations.virtualDocuments.push({ scheme, provider });
        return disposable();
      },
    },
    languages: {
      createDiagnosticCollection() {
        const collection = new FakeDiagnosticCollection();
        registrations.diagnostics.push(collection);
        return collection;
      },
      registerDefinitionProvider(selector, provider) {
        registrations.definitions.push({ selector, provider });
        return disposable();
      },
      registerDocumentSemanticTokensProvider(selector, provider, legend) {
        registrations.semanticTokens.push({ selector, provider, legend });
        return disposable();
      },
    },
    window: {
      createOutputChannel() {
        const output = new FakeOutputChannel();
        registrations.outputs.push(output);
        return output;
      },
    },
  };
}

export class FakeDiagnosticCollection {
  constructor() {
    this.entries = [];
  }

  clear() { this.entries = []; }
  delete(uri) { this.entries = this.entries.filter((entry) => entry.uri.toString() !== uri.toString()); }
  dispose() {}

  set(uri, diagnostics) {
    this.entries.push({ uri, diagnostics });
  }
}

export class FakeOutputChannel {
  constructor() {
    this.lines = [];
  }

  append(message) {
    this.lines.push(message);
  }

  appendLine(message) {
    this.lines.push(message);
  }

  dispose() {}
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

export function fakeDocument({ uri, version, text }) {
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

export function frame(message) {
  const body = JSON.stringify(message);
  return Buffer.from(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}

export function parseRpcMessage(message) {
  const bodyStart = message.indexOf("\r\n\r\n") + 4;
  return JSON.parse(message.slice(bodyStart));
}

export const serverCapabilities = {
  definitionProvider: true,
  semanticTokensProvider: {
    full: true,
    legend: { tokenTypes: ["function", "variable"], tokenModifiers: ["readonly", "declaration"] },
  },
};

export async function initializeServer(server, process, capabilities = serverCapabilities) {
  process.stdout.emit("data", frame({ jsonrpc: "2.0", id: 1, result: { capabilities } }));
  await server.ready;
}
