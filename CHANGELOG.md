# Changelog

All notable changes to Redstart are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the release workflow
pulls the section matching each tag into the GitHub Release notes.

## [Unreleased]

## [0.9.0] - 2026-06-27

The optimising compiler begins — roadmap §4.3 (Lever 2).

### Added
- **Inferred `immutable` entities.** The checker now proves which entities are
  append-only — created but never loaded (`load`/`loadInBlock`/`loadOrCreate`) and
  never field-mutated anywhere — and codegen emits `@entity(immutable: true)` for
  them automatically. Immutable entities index faster and use far less disk
  (Edge & Node benchmark: up to 19% faster / 48% less disk). `redstart build`
  reports each inference (`⚡ inferred @entity(immutable: true) for …`).
  Conservative by construction; consistent with the gate-proven hand-annotated
  immutability on the ERC-20 fixture.
- `Generated.notes` — informational optimisation notes, separate from warnings.

## [0.8.0] - 2026-06-27

Division footguns — roadmap §3.4 / §3.5.

### Added
- **E090** — dividing by a statically-zero value (`/ 0`, `/ 0.0`,
  `/ BigInt.zero()`, `/ BigDecimal.zero()`) is now a compile error. A divide-by-
  zero is a *fatal, deterministic sync halt* (`attempted to divide … by zero`).
- **W030** — assigning a `BigInt / BigInt` result to a `BigDecimal` field is
  flagged: integer division truncates the fraction first (the canonical Uniswap
  "price returns 0" bug). Use `.divDecimal()`. Integer division into integer
  fields is unaffected (no false positives).
- `redstart explain` covers both new codes.

## [0.7.0] - 2026-06-27

The store-diff kill-gate is **GREEN**, and a deploy footgun is gone.

### Added
- **Store-diff conformance gate proven.** `conformance/fixtures/arb-erc20` (the
  ARB token on Arbitrum One) deployed to a live graph-node alongside the
  hand-written reference and produced a **byte-identical store** — 0 diffs across
  10 Account + 13 Transfer entities at block 477,660,492. Indexing fidelity, the
  roadmap's #1 risk, is no longer a hypothesis.

### Fixed
- **ABI normalisation on build.** Emitted ABIs now always carry `anonymous` on
  event entries. graph-node *requires* it at deploy time (even though `graph
  build` doesn't), so an ABI missing it caused a cryptic `graph deploy` failure.
  Redstart now adds it automatically.

## [0.6.0] - 2026-06-27

Performance lint: the "stuck at 3%" eth_call — roadmap §4.2.

### Added
- **W020** — a contract call (`eth_call`) inside a `for`/`while` loop is now
  flagged. Each call is a 100 ms+ blocking RPC run serially while the handler is
  paused, so N iterations means N round-trips — the classic "stuck at 3%" sync.
  Calls outside loops are unaffected (no false positives). `redstart explain
  W020` documents it.

## [0.5.0] - 2026-06-26

Handler-shape lints + warning-severity diagnostics — roadmap §4.7.

### Added
- **Warning-severity diagnostics.** Diagnostics can now be warnings (lints):
  they're reported but don't fail the build. `check --json` carries the real
  `severity`; `redstart check` prints warnings and still exits 0.
- **W010** — a `handler call` on a network without Parity call tracing
  (Arbitrum, Optimism, Base, Polygon, BNB, …), where the handler silently never
  fires. Prefer an event handler.
- **W011** — an unfiltered block handler (runs on every block of the whole
  chain). Add `every N` or `once`.
- `redstart explain` covers both new codes.

Determinism by construction — roadmap §3.7.

### Added
- **E080** — calling a non-deterministic host function (`Date.now`, `Date.UTC`,
  `Date.parse`, `Math.random`) is now a compile error, with a fix-it pointing at
  `event.block.timestamp`. Non-determinism diverges Proof-of-Indexing across
  indexers (a slashing risk); graph-node only blocks some of these at runtime.
  `redstart explain E080` documents it.

## [0.3.0] - 2026-06-26

Errors that teach — the second roadmap step (§5.5).

### Added
- `redstart explain <CODE>` — explains any diagnostic code: title, what
  triggered it, **the bug it prevents**, and the canonical fix. Supports
  `--json`; bare `redstart explain` lists all 24 codes.
- A diagnostic-code registry in `redstart-checker` (`explain` module), the
  shared source of truth for code explanations.

## [0.2.0] - 2026-06-26

First step of the [2026 roadmap](docs/ROADMAP-2026.md) — agent-native diagnostics.

### Added
- `redstart check --json` — machine-readable diagnostics for editors and agent
  loops: `{ "ok": bool, "diagnostics": [ {code, severity, message, label, help,
  file, line, column, offset, length} ] }` on stdout, non-zero exit when not ok.
  Lex/parse/resolution failures are emitted too (ANSI stripped). (Roadmap §5.1.)
- `Diag` now carries 1-indexed `line`/`column` and exposes `code_short()`
  (e.g. `E062`) and `label_str()`.

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
