// Minimal VS Code client: starts `redstart lsp` and wires it to .red files.
const { workspace } = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const serverPath = workspace.getConfiguration("redstart").get("serverPath") || "redstart";

  const serverOptions = {
    run: { command: serverPath, args: ["lsp"], transport: TransportKind.stdio },
    debug: { command: serverPath, args: ["lsp"], transport: TransportKind.stdio },
  };

  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "redstart" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.red"),
    },
  };

  client = new LanguageClient("redstart", "Redstart Language Server", serverOptions, clientOptions);
  client.start();
  context.subscriptions.push(client);
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
