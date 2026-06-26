# Changelog

All notable changes to Redstart are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the release workflow
pulls the section matching each tag into the GitHub Release notes.

## [Unreleased]

## [0.1.0] - 2026-06-26

Stage 0 — foundations. The first end-to-end vertical slice.

### Added
- Lexer + recursive-descent parser with `miette` diagnostics (`redstart-parser`).
- `redstart.toml` manifest + multi-file module tree with cycle detection (`redstart-loader`).
- Semantic checker: nullable-deref, derived back-refs, required-field init,
  arithmetic-on-`Option`, `.value`-without-`match` (`redstart-checker`).
- AssemblyScript lowering + `schema.graphql` / `subgraph.yaml` generation,
  eject-verified through canonical `graph build` to WASM (`redstart-codegen`).
- Event, call, block, and file handlers; dynamic data sources (templates);
  timeseries entities + aggregations; enums, interfaces, derived fields.
- Native test interpreter — mock store and call mocking, no WASM/Docker
  (`redstart-test`).
- CLI: `new`, `check`, `build`, `test`, `fmt`, `dev`, `deploy` (`redstart-cli`).
- Language server with diagnostics, formatting, hover, go-to-def, completion
  (`redstart-lsp`); tree-sitter grammar + VS Code extension.
- Conformance harness: field-level store-diff against reference subgraphs.
- Release pipeline: cross-compiled binaries (macOS arm64/x86_64,
  Linux x86_64/arm64) to GitHub Releases + Homebrew tap.

[Unreleased]: https://github.com/lodestar-team/redstart/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lodestar-team/redstart/releases/tag/v0.1.0
