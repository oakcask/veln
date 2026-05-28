const childProcess = require("child_process");
const vscode = require("vscode");

const tokenTypes = [
  "namespace",
  "type",
  "parameter",
  "variable",
  "property",
  "enumMember",
  "function",
  "keyword",
  "comment",
  "string",
  "number",
  "operator",
];

const tokenModifiers = [
  "declaration",
  "readonly",
  "defaultLibrary",
  "test",
  "result",
  "hole",
];

class VelnLanguageServer {
  constructor(command, args, cwd, output, trace, onDiagnostics, onClearDiagnostics) {
    this.nextId = 1;
    this.pending = new Map();
    this.buffer = Buffer.alloc(0);
    this.output = output;
    this.trace = trace;
    this.onDiagnostics = onDiagnostics;
    this.onClearDiagnostics = onClearDiagnostics;
    this.syncedDocuments = new Map();
    this.traceLine(
      `starting server command=${JSON.stringify(command)} args=${JSON.stringify(args)} cwd=${JSON.stringify(cwd)}`,
    );
    this.process = childProcess.spawn(command, args, {
      cwd,
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.process.stdout.on("data", (chunk) => this.read(chunk));
    this.process.stderr.on("data", (chunk) => {
      this.output.append(`veln lsp: ${chunk.toString()}`);
    });
    this.process.on("error", (error) => {
      this.output.appendLine(`Failed to start Veln language server: ${error.message}`);
      this.onClearDiagnostics();
      for (const { reject } of this.pending.values()) {
        reject(error);
      }
      this.pending.clear();
    });
    this.process.on("exit", (code, signal) => {
      this.traceLine(`server exited code=${code} signal=${signal}`);
      this.onClearDiagnostics();
      for (const { reject } of this.pending.values()) {
        reject(new Error("Veln language server exited"));
      }
      this.pending.clear();
    });
    this.sendRequest("initialize", {
      capabilities: {},
    }).then(() => this.sendNotification("initialized", {}));
  }

  dispose() {
    this.onClearDiagnostics();
    this.sendRequest("shutdown", {})
      .catch(() => undefined)
      .finally(() => {
        this.sendNotification("exit", {});
      });
  }

  syncDocument(document) {
    const uri = document.uri.toString();
    if (this.syncedDocuments.get(uri) === document.version) {
      return;
    }
    const textDocument = {
      uri,
      languageId: "veln",
      version: document.version,
      text: document.getText(),
    };
    if (this.syncedDocuments.has(uri)) {
      this.sendNotification("textDocument/didChange", {
        textDocument: { uri, version: document.version },
        contentChanges: [{ text: textDocument.text }],
      });
    } else {
      this.sendNotification("textDocument/didOpen", { textDocument });
    }
    this.syncedDocuments.set(uri, document.version);
  }

  closeDocument(document) {
    const uri = document.uri.toString();
    if (!this.syncedDocuments.has(uri)) {
      return;
    }
    this.syncedDocuments.delete(uri);
    this.sendNotification("textDocument/didClose", {
      textDocument: { uri },
    });
  }

  semanticTokens(document) {
    this.syncDocument(document);
    return this.sendRequest("textDocument/semanticTokens/full", {
      textDocument: { uri: document.uri.toString() },
    });
  }

  sendNotification(method, params) {
    const message = {
      jsonrpc: "2.0",
      method,
      params,
    };
    this.traceMessage("client", message);
    this.write(message);
  }

  sendRequest(method, params) {
    const id = this.nextId++;
    const message = {
      jsonrpc: "2.0",
      id,
      method,
      params,
    };
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.traceMessage("client", message);
      this.write(message);
    });
  }

  write(message) {
    if (!this.process.stdin.writable) {
      return;
    }
    const body = JSON.stringify(message);
    this.process.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
  }

  read(chunk) {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    while (true) {
      const headerEnd = this.buffer.indexOf("\r\n\r\n");
      if (headerEnd < 0) {
        return;
      }
      const header = this.buffer.slice(0, headerEnd).toString();
      const match = header.match(/Content-Length:\s*(\d+)/i);
      if (!match) {
        this.buffer = Buffer.alloc(0);
        return;
      }
      const length = Number(match[1]);
      const bodyStart = headerEnd + 4;
      const bodyEnd = bodyStart + length;
      if (this.buffer.length < bodyEnd) {
        return;
      }
      const body = this.buffer.slice(bodyStart, bodyEnd).toString();
      this.buffer = this.buffer.slice(bodyEnd);
      let message;
      try {
        message = JSON.parse(body);
      } catch (error) {
        this.output.appendLine(`Failed to parse Veln language server message: ${error.message}`);
        continue;
      }
      this.traceMessage("server", message);
      this.handleMessage(message);
    }
  }

  handleMessage(message) {
    if (message.method === "textDocument/publishDiagnostics") {
      this.onDiagnostics(message.params);
      return;
    }
    this.handleResponse(message);
  }

  handleResponse(message) {
    const pending = this.pending.get(message.id);
    if (!pending) {
      return;
    }
    this.pending.delete(message.id);
    if (message.error) {
      pending.reject(new Error(message.error.message));
    } else {
      pending.resolve(message.result);
    }
  }

  traceLine(line) {
    if (this.trace === "off") {
      return;
    }
    this.output.appendLine(`[lsp] ${line}`);
  }

  traceMessage(direction, message) {
    if (this.trace === "off") {
      return;
    }
    if (this.trace === "verbose") {
      this.output.appendLine(`[lsp:${direction}] ${summarizeJson(message)}`);
      return;
    }
    this.output.appendLine(`[lsp:${direction}] ${summarizeMessage(message)}`);
  }
}

function summarizeMessage(message) {
  const id = message.id === undefined ? "" : ` id=${message.id}`;
  if (message.method) {
    return `${message.method}${id}`;
  }
  if (message.error) {
    return `response${id} error=${message.error.message}`;
  }
  if (message.result?.data) {
    return `response${id} result.data.length=${message.result.data.length}`;
  }
  return `response${id} ok`;
}

function summarizeJson(value) {
  const json = JSON.stringify(redactLargeText(value));
  if (json.length <= 4000) {
    return json;
  }
  return `${json.slice(0, 4000)}...`;
}

function redactLargeText(value) {
  if (Array.isArray(value)) {
    return value.map((item) => redactLargeText(item));
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, child]) => [
        key,
        key === "text" && typeof child === "string"
          ? `<${child.length} chars>`
          : redactLargeText(child),
      ]),
    );
  }
  return value;
}

function workspaceFolderPath() {
  return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

function resolveServerCommand(command) {
  const workspaceFolder = workspaceFolderPath();
  if (!workspaceFolder) {
    return command;
  }
  return command.replaceAll("${workspaceFolder}", workspaceFolder);
}

function resolveServerArguments(args) {
  if (!Array.isArray(args)) {
    return ["lsp"];
  }
  return args.map((arg) => resolveServerCommand(arg));
}

function applyDiagnostics(collection, params) {
  const uri = vscode.Uri.parse(params.uri);
  const diagnostics = (params.diagnostics ?? []).map((diagnostic) => {
    const item = new vscode.Diagnostic(
      toRange(diagnostic.range),
      diagnostic.message ?? "",
      toDiagnosticSeverity(diagnostic.severity),
    );
    item.code = diagnostic.code;
    item.source = diagnostic.source ?? "veln";
    return item;
  });
  collection.set(uri, diagnostics);
}

function toRange(range) {
  const start = range?.start ?? {};
  const end = range?.end ?? start;
  return new vscode.Range(
    new vscode.Position(
      Math.max(0, start.line ?? 0),
      Math.max(0, start.character ?? 0),
    ),
    new vscode.Position(
      Math.max(0, end.line ?? start.line ?? 0),
      Math.max(0, end.character ?? start.character ?? 0),
    ),
  );
}

function toDiagnosticSeverity(severity) {
  switch (severity) {
    case 1:
      return vscode.DiagnosticSeverity.Error;
    case 2:
      return vscode.DiagnosticSeverity.Warning;
    case 3:
      return vscode.DiagnosticSeverity.Information;
    case 4:
      return vscode.DiagnosticSeverity.Hint;
    default:
      return vscode.DiagnosticSeverity.Error;
  }
}

function activate(context) {
  const output = vscode.window.createOutputChannel("Veln");
  const diagnostics = vscode.languages.createDiagnosticCollection("veln");
  context.subscriptions.push(output);
  context.subscriptions.push(diagnostics);
  const config = vscode.workspace.getConfiguration("veln");
  const command = config.get("server.path", "veln");
  const args = config.get("server.arguments", ["lsp"]);
  const trace = config.get("server.trace", "off");
  const server = new VelnLanguageServer(
    resolveServerCommand(command),
    resolveServerArguments(args),
    workspaceFolderPath(),
    output,
    trace,
    (params) => applyDiagnostics(diagnostics, params),
    () => diagnostics.clear(),
  );
  const legend = new vscode.SemanticTokensLegend(tokenTypes, tokenModifiers);
  const provider = {
    async provideDocumentSemanticTokens(document) {
      const result = await server.semanticTokens(document);
      return new vscode.SemanticTokens(new Uint32Array(result.data));
    },
  };
  context.subscriptions.push(server);
  for (const document of vscode.workspace.textDocuments) {
    if (document.languageId === "veln") {
      server.syncDocument(document);
    }
  }
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((document) => {
      if (document.languageId === "veln") {
        server.syncDocument(document);
      }
    }),
  );
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((event) => {
      if (event.document.languageId === "veln") {
        server.syncDocument(event.document);
      }
    }),
  );
  context.subscriptions.push(
    vscode.workspace.onDidCloseTextDocument((document) => {
      if (document.languageId === "veln") {
        server.closeDocument(document);
        diagnostics.delete(document.uri);
      }
    }),
  );
  context.subscriptions.push(
    vscode.languages.registerDocumentSemanticTokensProvider(
      { language: "veln" },
      provider,
      legend,
    ),
  );
}

function deactivate() {}

module.exports = {
  activate,
  deactivate,
  _test: {
    VelnLanguageServer,
    applyDiagnostics,
    summarizeMessage,
    summarizeJson,
    toDiagnosticSeverity,
  },
};
