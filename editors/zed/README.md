# Redstart for Zed

Syntax highlighting (tree-sitter) and language-server support (`redstart lsp`) for
`.red` files in [Zed](https://zed.dev).

## Install

Once published, search **Redstart** in Zed's extension registry
(`zed: extensions`).

### Dev install (from this repo)

1. Build the toolchain so `redstart` is on your `PATH`
   (`cargo install --path crates/redstart-cli`, or grab a release binary).
2. In Zed: **`zed: install dev extension`** and pick this `editors/zed` directory.

Zed fetches the tree-sitter grammar from
[`tree-sitter-redstart`](../../tree-sitter-redstart) (the `path` in
`extension.toml`), compiles it, applies the highlight queries in
`languages/redstart/`, and launches `redstart lsp` for diagnostics, hover,
go-to-definition, and completion.

## Layout

```
extension.toml                     # manifest: grammar + language server
Cargo.toml, src/lib.rs             # tiny wasm extension that launches `redstart lsp`
languages/redstart/config.toml     # comments, brackets, file suffix
languages/redstart/highlights.scm  # tree-sitter highlight queries
```

The highlight queries are kept in sync with
[`tree-sitter-redstart/queries/highlights.scm`](../../tree-sitter-redstart/queries/highlights.scm).
