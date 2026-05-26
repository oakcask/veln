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
  constructor(command, args, cwd, output) {
    this.nextId = 1;
    this.pending = new Map();
    this.buffer = Buffer.alloc(0);
    this.output = output;
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
      for (const { reject } of this.pending.values()) {
        reject(error);
      }
      this.pending.clear();
    });
    this.process.on("exit", () => {
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
    this.sendRequest("shutdown", {})
      .catch(() => undefined)
      .finally(() => {
        this.sendNotification("exit", {});
      });
  }

  semanticTokens(document) {
    const textDocument = {
      uri: document.uri.toString(),
      languageId: "veln",
      version: document.version,
      text: document.getText(),
    };
    this.sendNotification("textDocument/didOpen", { textDocument });
    return this.sendRequest("textDocument/semanticTokens/full", {
      textDocument: { uri: textDocument.uri },
    });
  }

  sendNotification(method, params) {
    this.write({
      jsonrpc: "2.0",
      method,
      params,
    });
  }

  sendRequest(method, params) {
    const id = this.nextId++;
    this.write({
      jsonrpc: "2.0",
      id,
      method,
      params,
    });
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
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
      this.handleResponse(JSON.parse(body));
    }
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

function activate(context) {
  const output = vscode.window.createOutputChannel("Veln");
  context.subscriptions.push(output);
  const command = vscode.workspace
    .getConfiguration("veln")
    .get("server.path", "veln");
  const args = vscode.workspace
    .getConfiguration("veln")
    .get("server.arguments", ["lsp"]);
  const server = new VelnLanguageServer(
    resolveServerCommand(command),
    resolveServerArguments(args),
    workspaceFolderPath(),
    output,
  );
  const legend = new vscode.SemanticTokensLegend(tokenTypes, tokenModifiers);
  const provider = {
    async provideDocumentSemanticTokens(document) {
      const result = await server.semanticTokens(document);
      return new vscode.SemanticTokens(new Uint32Array(result.data));
    },
  };
  context.subscriptions.push(server);
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
};
