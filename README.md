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
| Semantic checker — unknown source/event/type, missing source settings, `derived` back-refs, required-field init, `.value`-without-`match`, arithmetic-on-`Option`, assign-to-`derived` | `redstart-checker` | ✅ working |
| `redstart fmt` — canonical, comment-preserving formatting (`--check` mode) | `redstart-cli` | ✅ working |
| Tree-sitter grammar + highlight queries (Neovim/Helix/Zed/GitHub) | `tree-sitter-redstart` | ✅ grammar written |
| `dev` watch loop, in-language `test` → Matchstick, LSP | `redstart-cli` | ⏳ later stages |

The AssemblyScript lowering is the whole bet: the **kill/pivot threshold** is a
field-level store-diff against canonical subgraph deployments. That's the next
milestone.

## Try it

```sh
cargo run -p redstart-cli -- new my-subgraph
cargo run -p redstart-cli -- build my-subgraph

# or against the worked example (split across two modules):
cargo run -p redstart-cli -- check examples/erc20
cargo run -p redstart-cli -- build examples/erc20
cargo run -p redstart-cli -- fmt --check examples/erc20
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

## Architecture

A small, batteries-included, single-binary toolchain (the Gleam/Elm/Prisma
model). Crates are split by compiler phase:

```
redstart-parser   lex → AST  (source of all spans & diagnostics)
redstart-loader   redstart.toml + `mod` resolution → ModuleTree
redstart-checker  ModuleTree → semantic analysis → Checked symbol table (RTy/ABI)
redstart-codegen  ModuleTree + Checked → schema.graphql, subgraph.yaml, mappings.ts
redstart-cli      the `redstart` binary: new / check / build (…dev/test/fmt/lsp)
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
