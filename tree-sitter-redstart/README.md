# tree-sitter-redstart

A [tree-sitter](https://tree-sitter.github.io) grammar for the
[Redstart](../README.md) subgraph language, for syntax highlighting in
Neovim, Helix, Zed, and on GitHub.

## Build

Requires the tree-sitter CLI (`npm install -g tree-sitter-cli`):

```sh
cd tree-sitter-redstart
tree-sitter generate     # builds the parser from grammar.js
tree-sitter test         # runs corpus tests (see test/)
tree-sitter parse ../examples/erc20/src/main.red   # sanity-check parsing
```

## Editor setup

- **Helix / Zed / Neovim (nvim-treesitter):** point the editor's grammar config
  at this directory and copy `queries/highlights.scm` into the runtime queries
  path for the `redstart` language.
- **File type:** associate `*.red` with the `redstart` language.

## Status

The grammar tracks the hand-rolled parser in `crates/redstart-parser`. When the
language grammar changes there, update `grammar.js` to match. It has not yet been
generated/tested in CI (needs a Node toolchain); `grammar.js` and the highlight
queries are the source of truth.
