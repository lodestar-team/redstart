# Redstart

**A multi-file language for authoring The Graph subgraphs.**

Today a subgraph is three loosely-coupled artifacts — `schema.graphql`,
`subgraph.yaml`, and AssemblyScript mappings — stitched together by stringly-typed
names and a manual `graph codegen` step. Drift between them is the dominant source
of *"it compiled but failed at runtime, three hours into a sync."*

Redstart unifies all three into one language — split across as many `.red` modules
as you like (`mod`/`use`, just like Rust) — type-checks them against each other,
and transpiles to readable AssemblyScript that the canonical `graph build`
toolchain compiles unmodified. Entities can live in one module and the handlers
that write them in another; the compiler resolves and checks across all of them. The entire class of AssemblyScript footguns —
nullable-arithmetic miscompiles, `==`/`===` inversion, reverted-call aborts,
array prefill, forgotten `.save()` — becomes **unrepresentable by construction**.

```redstart
abi ERC20 from "./abis/ERC20.json"

entity Account {
  id: Id<Bytes>
  balance: BigInt
  label: Option<String>          // nullability is always explicit — there is no `null`
  transfersOut: [Transfer] derived from from
}

entity Transfer immutable {
  id: Id<Bytes>
  from: Account
  to: Account
  value: BigInt
  timestamp: BigInt
}

source Token {
  abi: ERC20
  network: mainnet
  address: 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
  startBlock: 6082465
}

handler on Token.Transfer(event) {
  let receiver = Account.loadOrCreate(event.params.to, { balance: BigInt.zero })
  receiver.balance = receiver.balance + event.params.value
  // auto-saved at handler end (dirty-tracked) — forgetting `.save()` can't happen
}
```

`redstart build` turns that into `schema.graphql` + `subgraph.yaml` +
`mappings.ts`. The event signature in the manifest
(`Transfer(indexed address,indexed address,uint256)`) is derived from the ABI by
reference — rename the event and it's a *compile* error, not a runtime one.

## Why

The killer feature is **unification, not syntax**. A single source of truth makes
manifest/schema/handler drift impossible. The **eject path** — readable emitted
AssemblyScript the canonical toolchain consumes unmodified — means abandoning
Redstart costs nothing but the generated code, which keeps working. That defuses
the bus-factor objection to betting production infra on one language.

Redstart does **not** make indexing faster; it makes *staying on The Graph's
decentralized network pleasant*. It is scoped as a Graph-Foundation-grant public
good in the lineage of Matchstick, not a venture bet.

## Status

🚧 **Stage 0 — foundations.** Early but real and end-to-end.

| Component | Crate | State |
|---|---|---|
| Lexer + parser (`logos` + recursive descent, `miette` diagnostics) | `redstart-parser` | ✅ working |
| `redstart.toml` manifest + multi-file module tree (cycle detection) | `redstart-loader` | ✅ working |
| `schema.graphql` + `subgraph.yaml` generation from the unified AST | `redstart-codegen` | ✅ working |
| AssemblyScript mapping lowering — `loadOrCreate`, `BigInt`/`BigDecimal` operators, auto-save dirty-tracking, contract calls (`Result` → `try_*`), `match` | `redstart-codegen` | ✅ vertical slice (ERC-20) |
| Control flow — `if`/`else if`/`else`, `while`, `for` (numeric ranges + list iteration), array literals & indexing, lowered to native AssemblyScript | `redstart-codegen` | ✅ working |
| Handler kinds — event (`handler on Src.Event`), call (`handler call Src.fn`), and block (`handler block Src [every N\|once]`) → `eventHandlers`/`callHandlers`/`blockHandlers` | `redstart-codegen` | ✅ working |
| Dynamic data sources — `template` blocks + `<Template>.create(addr)` / `.createWithContext(addr, ctx)` and `DataSourceContext`, the factory pattern | `redstart-codegen` | ✅ working |
| Semantic checker — unknown source/event/type, missing source settings, `derived` back-refs, required-field init, `.value`-without-`match`, arithmetic-on-`Option`, assign-to-`derived` | `redstart-checker` | ✅ working |
| `redstart test` — native test interpreter (mock store + mocked calls, no WASM/Docker/Matchstick) | `redstart-test` | ✅ working |
| `redstart fmt` — canonical, comment-preserving formatting (`--check` mode) | `redstart-cli` | ✅ working |
| `redstart dev` — watch loop re-running check → build → test on every change | `redstart-cli` | ✅ working |
| Tree-sitter grammar + highlight queries (Neovim/Helix/Zed/GitHub) | `tree-sitter-redstart` | ✅ grammar written |
| `redstart lsp` — language server: diagnostics, formatting, symbols, hover, go-to-def, completion | `redstart-lsp` | ✅ working |
| VS Code extension (LSP client + TextMate highlighting) | `editors/vscode` | ✅ working |

The AssemblyScript lowering is the whole bet: the **kill/pivot threshold** is a
field-level store-diff against canonical subgraph deployments. The harness for it
lives in [`conformance/`](conformance/) — `./conformance/run.sh build` proves the
eject path (canonical `graph build` compiles our output unmodified) with only
Node; `run.sh all` deploys our subgraph alongside an idiomatic hand-written
reference and store-diffs them at a fixed block.

> **✅ Eject path proven.** `graph codegen` + `graph build` compile the generated
> ERC-20 subgraph unmodified into WebAssembly (`build/Token/Token.wasm`). The
> canonical toolchain accepts Redstart's output with zero manual edits.

## Try it

```sh
cargo run -p redstart-cli -- new my-subgraph
cargo run -p redstart-cli -- build my-subgraph

# or against the worked example (split across two modules):
cargo run -p redstart-cli -- check examples/erc20
cargo run -p redstart-cli -- build examples/erc20
cargo run -p redstart-cli -- test examples/erc20
cargo run -p redstart-cli -- fmt --check examples/erc20
cargo run -p redstart-cli -- dev examples/erc20    # watch: check → build → test on save
```

## Project layout

A project is a `redstart.toml` plus a tree of `.red` modules. The entry module
pulls in others with `mod`; any module can reference another's declarations.

```
my-subgraph/
  redstart.toml        # [project] name / entry / out_dir
  src/
    main.red           # mod accounts;  +  abi / source / handler
    accounts.red       # entity Account, entity Transfer
    abis/ERC20.json
  build/               # generated: schema.graphql, subgraph.yaml, src/mappings.ts, abis/
```

`mod accounts;` resolves to `accounts.red` (or `accounts/mod.red`), exactly like
Rust. The example's `Token.Transfer` handler in `main.red` loads and writes the
`Account` and `Transfer` entities declared in `accounts.red` — across modules,
type-checked, no drift.

## Testing

`redstart test` runs your `test` blocks **natively** — a tree-walking interpreter
evaluates handler ASTs against an in-memory mock store. No WASM compile, no
downloaded Matchstick binary, no Docker, and — because tests are written in
Redstart, not AssemblyScript — no `matchstick-as`/`graph-ts` version skew. Event
fixtures are synthesised from a record literal; contract reads are mocked inline:

```redstart
test "a transfer debits the sender and credits the receiver" {
  Token.Transfer({ from: 0x01, to: 0x02, value: 100 })
  assertEq(Account.at(0x02).balance, 100)
  assert(Account.at(0x01).balance < 0)
}

test "approval writes the balance read via a contract call" {
  mockCall(ERC20.balanceOf(0x05), 4200)        // mock the eth_call
  Token.Approval({ owner: 0x05, spender: 0x06, value: 1 })
  assertEq(Account.at(0x05).balance, 4200)
}
```

This is the fast inner loop for *handler logic*. Fidelity to the real compiled
WASM is the job of the [conformance gate](conformance/), which store-diffs a real
graph-node deployment against a canonical reference. Two layers, two concerns.

## Architecture

A small, batteries-included, single-binary toolchain (the Gleam/Elm/Prisma
model). Crates are split by compiler phase:

```
redstart-parser   lex → AST  (source of all spans & diagnostics)
redstart-loader   redstart.toml + `mod` resolution → ModuleTree
redstart-checker  ModuleTree → semantic analysis → Checked symbol table (RTy/ABI)
redstart-codegen  ModuleTree + Checked → schema.graphql, subgraph.yaml, mappings.ts
redstart-test     ModuleTree → native interpreter for `test` blocks (mock store)
redstart-lsp      tower-lsp language server (diagnostics/format/symbols/hover/def)
redstart-cli      the `redstart` binary: new / check / build / test / dev / fmt / lsp
```

The resolved type system (`RTy`, ABI reading) lives in `redstart-checker` and is
shared with codegen, so "what type is this expression" is answered in exactly one
place.

## Design principles (ranked)

1. **Impossible states unrepresentable** — every documented AS footgun is a type
   error or absent from the grammar.
2. **One source of truth** — schema, manifest, and mappings are one language.
3. **Errors teach** — Elm-grade diagnostics are the product.
4. **Feels like the domain** — Solidity-event affinity, entity-centric blocks.
5. **One obvious way** — no `==`/`===`, no integer-type zoo in the surface syntax.
6. **Always ejectable** — emitted AssemblyScript is readable and canonical.

## License

MIT © The Lodestar Team
