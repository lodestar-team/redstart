# Redstart for VS Code

Language support for Redstart subgraph files (`.red`): live diagnostics,
formatting, document outline, hover, go-to-definition, and completion — backed by
the `redstart lsp` language server, plus TextMate highlighting.

## Develop

```sh
cd editors/vscode
npm install
code .            # then press F5 to launch an Extension Development Host
```

Ensure the `redstart` binary is on your `PATH` (or set `redstart.serverPath` in
settings to an absolute path, e.g. `target/release/redstart`).

## Package

```sh
npm install -g @vscode/vsce
vsce package      # produces redstart-0.1.0.vsix
```

The extension shells out to `redstart lsp` over stdio — the same server used by
any LSP-capable editor (Neovim, Helix, Zed).
